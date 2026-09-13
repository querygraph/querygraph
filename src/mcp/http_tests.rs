use super::*;
use axum::body::Body;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn message(method: &str) -> Value {
    json!({"jsonrpc":"2.0","id":1,"method":method,"params":{"_meta":{
        modern::VERSION_KEY:modern::VERSION,"io.modelcontextprotocol/clientCapabilities":{}}}})
}

fn post_request(message: Value) -> axum::http::Request<Body> {
    let mut request = axum::http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", modern::VERSION)
        .header("mcp-method", message["method"].as_str().unwrap());
    if let Some(name) = message["params"]["name"].as_str() {
        request = request.header("mcp-name", name);
    }
    request.body(Body::from(message.to_string())).unwrap()
}

async fn body(response: Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

#[tokio::test]
async fn http_discovery_is_stateless_and_validates_headers_origin_and_methods() {
    let router = McpServer::new().http_router(vec!["https://console.example".into()]);
    let response = router
        .clone()
        .oneshot(post_request(message("server/discover")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.headers().contains_key("mcp-session-id"));
    assert_eq!(body(response).await["result"]["resultType"], "complete");
    for header in ["mcp-protocol-version", "mcp-method"] {
        let mut request = post_request(message("tools/list"));
        request.headers_mut().remove(header);
        let response = router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body(response).await["error"]["code"], -32020);
    }
    let mut request = post_request(message("tools/list"));
    request
        .headers_mut()
        .insert("origin", "https://evil.example".parse().unwrap());
    assert_eq!(
        router.clone().oneshot(request).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let mut request = post_request(message("tools/list"));
    request
        .headers_mut()
        .insert("mcp-method", "tools/call".parse().unwrap());
    assert_eq!(
        body(router.clone().oneshot(request).await.unwrap()).await["error"]["code"],
        -32020
    );
    for method in ["GET", "DELETE"] {
        let request = axum::http::Request::builder()
            .method(method)
            .uri("/mcp")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            router.clone().oneshot(request).await.unwrap().status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
    }
    let response = router
        .oneshot(post_request(message("unknown")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(body(response).await["error"]["code"], -32601);
}

#[tokio::test]
async fn http_checks_metadata_and_base64_names_and_returns_scoped_sse() {
    let router = McpServer::new().http_router(vec![]);
    let mut missing = message("tools/list");
    missing["params"]["_meta"]
        .as_object_mut()
        .unwrap()
        .remove("io.modelcontextprotocol/clientCapabilities");
    assert_eq!(
        body(router.clone().oneshot(post_request(missing)).await.unwrap()).await["error"]["code"],
        -32602
    );
    let mut wrong = message("tools/list");
    wrong["params"]["_meta"][modern::VERSION_KEY] = json!("2099-01-01");
    let mut request = post_request(wrong);
    request
        .headers_mut()
        .insert("mcp-protocol-version", "2099-01-01".parse().unwrap());
    assert_eq!(
        body(router.clone().oneshot(request).await.unwrap()).await["error"]["code"],
        -32022
    );
    let mut call = message("tools/call");
    call["params"]["name"] = json!("discover_pinax_ontology");
    call["params"]["arguments"] = json!({});
    let mut request = post_request(call.clone());
    request.headers_mut().insert(
        "mcp-name",
        format!("=?base64?{}?=", STANDARD.encode("discover_pinax_ontology"))
            .parse()
            .unwrap(),
    );
    request
        .headers_mut()
        .insert("mcp-session-id", "ignored-old-session".parse().unwrap());
    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    assert_eq!(response.headers()["x-accel-buffering"], "no");
    assert!(!response.headers().contains_key("mcp-session-id"));
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = std::str::from_utf8(&bytes).unwrap();
    let data = text
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .unwrap();
    let result: Value = serde_json::from_str(data).unwrap();
    assert_eq!(result["result"]["resultType"], "complete");
    assert_eq!(result["result"]["isError"], true);
    let mut missing_name = post_request(call);
    missing_name.headers_mut().remove("mcp-name");
    assert_eq!(
        body(router.oneshot(missing_name).await.unwrap()).await["error"]["code"],
        -32020
    );
}

#[test]
fn encoded_names_are_decoded_once_without_normalizing_content() {
    assert_eq!(
        decode_header("=?base64?SGVsbG8sIOS4lueVjA==?="),
        Some("Hello, 世界".into())
    );
    assert_eq!(
        decode_header("=?base64?bGluZTEKbGluZTI=?="),
        Some("line1\nline2".into())
    );
    assert_eq!(decode_header(" padded "), None);
    assert_eq!(decode_header("=?base64?bad!?="), None);
}
