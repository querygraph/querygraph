//! Stateless Streamable HTTP. A response stream owns its governed future and
//! concurrency permit; disconnect drops both. No session IDs or detached tasks.
use super::{MAX_MCP_MESSAGE_BYTES, McpServer, modern};
use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::post,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::Semaphore;

#[derive(Clone)]
struct HttpState {
    server: modern::Server,
    origins: Arc<Vec<String>>,
    permits: Arc<Semaphore>,
}

impl McpServer {
    /// Modern MCP endpoint at `/mcp`. Origins are exact, host-configured values;
    /// an empty allowlist rejects all browser origins. Pinax tools require
    /// their existing TypeDID signed intent on every request.
    pub fn http_router(&self, allowed_origins: Vec<String>) -> Router {
        let state = HttpState {
            server: self.modern.clone(),
            origins: Arc::new(allowed_origins),
            permits: Arc::new(Semaphore::new(16)),
        };
        Router::new()
            .route("/mcp", post(handle))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                validate_origin,
            ))
            .with_state(state)
    }

    /// Serve the modern endpoint on loopback, suitable for an SSH tunnel or
    /// an authenticated reverse proxy. Legacy stdio remains available.
    pub async fn run_http(
        &self,
        address: SocketAddr,
        allowed_origins: Vec<String>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            address.ip().is_loopback(),
            "MCP HTTP must bind to loopback; use an authenticated reverse proxy for remote access"
        );
        let listener = tokio::net::TcpListener::bind(address).await?;
        eprintln!(
            "QueryGraph MCP {} listening at http://{}/mcp",
            modern::VERSION,
            listener.local_addr()?
        );
        axum::serve(listener, self.http_router(allowed_origins)).await?;
        Ok(())
    }
}

async fn validate_origin(
    State(state): State<HttpState>,
    request: Request,
    next: axum::middleware::Next,
) -> Response {
    if request.headers().contains_key("origin")
        && single_header(request.headers(), "origin")
            .is_none_or(|origin| !state.origins.iter().any(|allowed| allowed == origin))
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    next.run(request).await
}

async fn handle(State(state): State<HttpState>, request: Request) -> Response {
    let permit = match state.permits.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => return StatusCode::TOO_MANY_REQUESTS.into_response(),
    };
    let (parts, body) = request.into_parts();
    if single_header(&parts.headers, "content-type").is_none_or(|value| {
        value.split(';').next().unwrap_or_default().trim() != "application/json"
    }) {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    if !accepts(&parts.headers, "application/json") || !accepts(&parts.headers, "text/event-stream")
    {
        return StatusCode::NOT_ACCEPTABLE.into_response();
    }
    let bytes = match tokio::time::timeout(
        Duration::from_secs(30),
        to_bytes(body, MAX_MCP_MESSAGE_BYTES),
    )
    .await
    {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(_)) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        Err(_) => return StatusCode::REQUEST_TIMEOUT.into_response(),
    };
    let message: Value = match serde_json::from_slice(&bytes) {
        Ok(message) => message,
        Err(_) => return failure(Value::Null, modern::Error::new(-32700, "parse error")),
    };
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    if !message.is_object()
        || message["jsonrpc"] != "2.0"
        || !message["method"].is_string()
        || message
            .get("params")
            .is_some_and(|params| !params.is_object())
    {
        return failure(Value::Null, modern::Error::new(-32600, "invalid request"));
    }
    // No client-to-server HTTP notifications are defined or advertised by this
    // implementation. Reject without echoing an ID or performing tool effects.
    if message.get("id").is_none() {
        return failure(
            Value::Null,
            modern::Error::new(-32600, "HTTP notifications are not supported"),
        );
    }
    if !(id.is_string() || id.is_i64() || id.is_u64()) {
        return failure(
            Value::Null,
            modern::Error::new(-32600, "invalid request ID"),
        );
    }
    let method = message["method"].as_str().unwrap_or_default().to_owned();
    let params = message.get("params").cloned().unwrap_or(json!({}));
    if let Err(error) = validate_headers(&parts.headers, &method, &params) {
        return failure(id, error);
    }
    let request = match modern::Request::parse(id.clone(), method, params) {
        Ok(request) => request,
        Err(error) => return failure(id, error),
    };
    if let Err(error) = request.validate_method() {
        return failure(id, error);
    }
    if !request.governed() {
        let result = state.server.dispatch_sync(&request);
        return match result {
            Ok(result) => json_response(StatusCode::OK, modern::complete(id, Ok(result))),
            Err(error) => failure(id, error),
        };
    }
    // Nothing is spawned: polling the SSE body polls the operation. Dropping
    // that body on disconnect cancels the operation before any further output.
    let stream = futures_util::stream::once(async move {
        let _permit = permit;
        let result =
            match tokio::time::timeout(Duration::from_secs(30), state.server.dispatch(&request))
                .await
            {
                Ok(result) => result,
                Err(_) => Err(modern::Error::new(1001, "request timed out")),
            };
        Ok::<Event, Infallible>(
            Event::default()
                .event("message")
                .data(modern::complete(request.id, result).to_string()),
        )
    });
    let mut response = Sse::new(stream)
        .keep_alive(KeepAlive::default().interval(Duration::from_secs(10)))
        .into_response();
    response
        .headers_mut()
        .insert("x-accel-buffering", "no".parse().expect("static header"));
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().expect("static header"));
    response
}

fn single_header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    Some(value)
}

fn accepts(headers: &HeaderMap, mime: &str) -> bool {
    headers
        .get_all("accept")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|value| {
            let mut parts = value.split(';');
            parts.next().is_some_and(|value| value.trim() == mime)
                && !parts.any(|part| {
                    part.trim()
                        .strip_prefix("q=")
                        .is_some_and(|q| q.parse::<f32>().is_ok_and(|q| q <= 0.0))
                })
        })
}

fn validate_headers(
    headers: &HeaderMap,
    method: &str,
    params: &Value,
) -> Result<(), modern::Error> {
    let mismatch = || {
        modern::Error::new(
            -32020,
            "HeaderMismatch: missing, malformed or inconsistent MCP headers",
        )
    };
    let version = single_header(headers, "mcp-protocol-version").ok_or_else(mismatch)?;
    // A present header with missing body metadata is a malformed params error;
    // a present but differing body value is a header mismatch.
    if let Some(body_version) = params["_meta"].get(modern::VERSION_KEY)
        && body_version.as_str() != Some(version)
    {
        return Err(mismatch());
    }
    if single_header(headers, "mcp-method") != Some(method) {
        return Err(mismatch());
    }
    let source = match method {
        "tools/call" | "prompts/get" => Some(&params["name"]),
        "resources/read" => Some(&params["uri"]),
        _ => None,
    };
    if let Some(source) = source {
        let encoded = single_header(headers, "mcp-name").ok_or_else(mismatch)?;
        let decoded = decode_header(encoded).ok_or_else(mismatch)?;
        if source.as_str() != Some(decoded.as_str()) {
            return Err(mismatch());
        }
    }
    Ok(())
}

fn decode_header(value: &str) -> Option<String> {
    if let Some(base64) = value
        .strip_prefix("=?base64?")
        .and_then(|s| s.strip_suffix("?="))
    {
        String::from_utf8(STANDARD.decode(base64).ok()?).ok()
    } else if value.trim() == value
        && value
            .bytes()
            .all(|byte| byte == b'\t' || (0x20..=0x7e).contains(&byte))
    {
        Some(value.to_owned())
    } else {
        None
    }
}

fn failure(id: Value, error: modern::Error) -> Response {
    let status = if error.code == -32601 {
        StatusCode::NOT_FOUND
    } else if error.code == -32603 {
        StatusCode::INTERNAL_SERVER_ERROR
    } else {
        StatusCode::BAD_REQUEST
    };
    json_response(status, error.response(id))
}

fn json_response(status: StatusCode, value: Value) -> Response {
    let mut response = (status, Json(value)).into_response();
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().expect("static header"));
    response
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
