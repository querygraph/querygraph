//! MCP 2026-07-28 over stateless stdio and Streamable HTTP.
//!
//! Modern requests carry version/capabilities independently and reference
//! imported models through explicit handles. Legacy stdio clients retain their
//! initialized semantic-model session. The Python CLI's Rust handoff serves
//! these same contracts without translating messages or request IDs.

use std::collections::BTreeMap;

use anyhow::Result;
use serde_json::{Value, json};

use crate::{
    navigator::AiNavigator,
    osi::OsiDocument,
    qglake::run_qglake_story,
    server::{answer_over_models, search_model},
};

mod http;
mod modern;
mod request;
mod transport;

pub use modern::VERSION as MCP_STATELESS_PROTOCOL_VERSION;

use request::ToolCall;

/// Latest handshake-based revision, retained for existing stdio callers.
/// Modern clients use [`MCP_STATELESS_PROTOCOL_VERSION`] per request.
pub const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

/// Maximum encoded JSON-RPC message size, including escaped registry documents.
pub const MAX_MCP_MESSAGE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Default)]
enum SessionState {
    #[default]
    Uninitialized,
    Initializing(ProtocolVersion),
    Ready(ProtocolVersion),
}

#[derive(Clone, Copy)]
enum ProtocolVersion {
    Legacy,
    Structured,
    Current,
}

impl ProtocolVersion {
    fn negotiate(requested: &str) -> Self {
        match requested {
            "2024-11-05" => Self::Legacy,
            "2025-06-18" => Self::Structured,
            _ => Self::Current,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Legacy => "2024-11-05",
            Self::Structured => "2025-06-18",
            Self::Current => MCP_PROTOCOL_VERSION,
        }
    }
}

/// Stateful MCP session: the semantic-model registry persists across calls.
#[derive(Default)]
pub struct McpServer {
    registry: BTreeMap<String, OsiDocument>,
    session: SessionState,
    registry_service: Option<std::sync::Arc<crate::registry_service::RegistryService>>,
    modern: modern::Server,
}

impl McpServer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a deployment-owned Pinax service. Tool arguments cannot replace it.
    pub fn with_registry_service(
        mut self,
        service: std::sync::Arc<crate::registry_service::RegistryService>,
    ) -> Self {
        self.modern.registry_service = Some(service.clone());
        self.registry_service = Some(service);
        self
    }

    /// Serve MCP over stdio until EOF.
    pub fn run_stdio(&mut self) -> Result<()> {
        let stdin = std::io::stdin();
        self.serve_io(
            &mut stdin.lock(),
            std::sync::Arc::new(std::sync::Mutex::new(std::io::stdout())),
        )
    }

    /// Handle a synchronous tool or lifecycle message. Governed I/O tools require
    /// [`Self::handle_line_async`]; notifications receive no reply.
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        match self.parse_line(line) {
            Incoming::Modern(request) => Some(
                modern::complete(request.id.clone(), self.modern.dispatch_sync(&request))
                    .to_string(),
            ),
            Incoming::Request { id, method, params } => {
                Some(encode_response(id, self.dispatch(&method, &params)))
            }
            Incoming::Reply(reply) => Some(reply),
            Incoming::Notification => None,
            Incoming::Cancelled(_) => None,
        }
    }

    /// Handle a message with cancellable governed backend I/O. The caller owns
    /// the future and timeout; no task is spawned and no request is retried.
    pub async fn handle_line_async(&mut self, line: &str) -> Option<String> {
        match self.parse_line(line) {
            Incoming::Modern(request) => Some(
                modern::complete(request.id.clone(), self.modern.dispatch(&request).await)
                    .to_string(),
            ),
            Incoming::Request { id, method, params } => {
                let result = if method == "tools/call"
                    && (params["name"] == "plan_pinax_scan"
                        || params["name"] == "execute_pinax_scan"
                        || params["name"] == "discover_pinax_tables"
                        || params["name"] == "discover_pinax_ontology")
                    && matches!(self.session, SessionState::Ready(_))
                {
                    self.call_registry_tool(&params).await
                } else {
                    self.dispatch(&method, &params)
                };
                Some(encode_response(id, result))
            }
            Incoming::Reply(reply) => Some(reply),
            Incoming::Notification => None,
            Incoming::Cancelled(_) => None,
        }
    }

    fn parse_line(&mut self, line: &str) -> Incoming {
        if line.len() > MAX_MCP_MESSAGE_BYTES {
            return Incoming::Reply(rpc_failure(
                Value::Null,
                -32600,
                "message exceeds size limit",
            ));
        }
        let message: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                return Incoming::Reply(
                    json!({
                        "jsonrpc": "2.0", "id": null,
                        "error": {"code": -32700, "message": format!("parse error: {error}")},
                    })
                    .to_string(),
                );
            }
        };
        if !message.is_object()
            || message["jsonrpc"] != "2.0"
            || !message["method"].is_string()
            || message
                .get("params")
                .is_some_and(|params| !params.is_object())
            || message
                .get("id")
                .is_some_and(|id| !id.is_null() && !id.is_string() && !id.is_i64() && !id.is_u64())
        {
            return Incoming::Reply(rpc_failure(Value::Null, -32600, "invalid request"));
        }
        let id = message.get("id").cloned();
        let method = message["method"].as_str().unwrap_or_default().to_string();
        let params = message.get("params").cloned().unwrap_or(json!({}));

        // Notifications (no id) get no response.
        let id = match id {
            Some(id) => id,
            None => {
                if method == "notifications/cancelled"
                    && (params["requestId"].is_string()
                        || params["requestId"].is_i64()
                        || params["requestId"].is_u64())
                    && params.get("reason").is_none_or(Value::is_string)
                {
                    return Incoming::Cancelled(params["requestId"].clone());
                }
                if method == "notifications/initialized"
                    && let SessionState::Initializing(version) = self.session
                {
                    self.session = SessionState::Ready(version);
                }
                return Incoming::Notification;
            }
        };

        if modern::recognizes(&method, &params) {
            return match modern::Request::parse(id.clone(), method, params) {
                Ok(request) => Incoming::Modern(request),
                Err(error) => Incoming::Reply(error.response(id).to_string()),
            };
        }
        Incoming::Request { id, method, params }
    }

    async fn call_registry_tool(&self, params: &Value) -> Result<Value, RpcError> {
        let result = async {
            let Some(service) = &self.registry_service else {
                return Err("registry backend is not configured".to_owned());
            };
            let arguments: request::SignedRegistryArguments =
                serde_json::from_value(params["arguments"].clone())
                    .map_err(|error| error.to_string())?;
            if params["name"] == "discover_pinax_ontology" {
                service
                    .discover_ontology(&arguments.intent, &arguments.envelope)
                    .await
                    .map_err(|error| registry_error_message(&error).to_owned())
            } else if params["name"] == "discover_pinax_tables" {
                let page = service
                    .discover(&arguments.intent, &arguments.envelope)
                    .await
                    .map_err(|error| registry_error_message(&error).to_owned())?;
                serde_json::to_value(page).map_err(|error| error.to_string())
            } else if params["name"] == "execute_pinax_scan" {
                let result = service
                    .execute_scan(&arguments.intent, &arguments.envelope)
                    .await
                    .map_err(|error| registry_error_message(&error).to_owned())?;
                serde_json::to_value(result).map_err(|error| error.to_string())
            } else {
                let prepared = service
                    .plan_scan(&arguments.intent, &arguments.envelope)
                    .await
                    .map_err(|error| registry_error_message(&error).to_owned())?;
                serde_json::to_value(prepared.summary()).map_err(|error| error.to_string())
            }
        }
        .await;
        let mut result = match result {
            Ok(value) => {
                json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":value,"isError":false})
            }
            Err(message) => json!({"content":[{"type":"text","text":message}],"isError":true}),
        };
        if matches!(self.session, SessionState::Ready(ProtocolVersion::Legacy)) {
            result
                .as_object_mut()
                .expect("tool result is an object")
                .remove("structuredContent");
        }
        Ok(result)
    }

    fn dispatch(&mut self, method: &str, params: &Value) -> Result<Value, RpcError> {
        if method != "initialize"
            && method != "ping"
            && !matches!(self.session, SessionState::Ready(_))
        {
            return Err(RpcError {
                code: -32002,
                message: "session is not initialized".into(),
            });
        }
        match method {
            "initialize" => {
                if !matches!(self.session, SessionState::Uninitialized) {
                    return Err(RpcError::invalid_params("session is already initialized"));
                }
                if !params["protocolVersion"].is_string()
                    || !params["capabilities"].is_object()
                    || !params["clientInfo"]["name"].is_string()
                    || !params["clientInfo"]["version"].is_string()
                {
                    return Err(RpcError::invalid_params(
                        "protocolVersion, capabilities, and clientInfo are required",
                    ));
                }
                let version = ProtocolVersion::negotiate(
                    params["protocolVersion"]
                        .as_str()
                        .ok_or_else(|| RpcError::invalid_params("protocolVersion is required"))?,
                );
                self.session = SessionState::Initializing(version);
                Ok(json!({
                "protocolVersion": version.as_str(),
                    "capabilities": {"tools": {}},
                    "serverInfo": {
                        "name": "querygraph",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                    "instructions": "QueryGraph governed semantic layer: import and \
                         search semantic models, build four-layer bundles, run the \
                         governed multi-agent story, answer questions with signed \
                         evidence chains, and verify TypeDID envelopes.",
                }))
            }
            "ping" => Ok(json!({})),
            "tools/list" => {
                if params.get("cursor").is_some() {
                    return Err(RpcError::invalid_params(
                        "this tool list has no continuation cursor",
                    ));
                }
                let mut definitions = tool_definitions();
                if matches!(self.session, SessionState::Ready(ProtocolVersion::Legacy)) {
                    for tool in definitions
                        .as_array_mut()
                        .expect("embedded tools are an array")
                    {
                        tool.as_object_mut()
                            .expect("embedded tool is an object")
                            .remove("outputSchema");
                        tool.as_object_mut()
                            .expect("embedded tool is an object")
                            .remove("annotations");
                    }
                }
                Ok(json!({"tools": definitions}))
            }
            "tools/call" => {
                let mut result = self.call_tool(params)?;
                if matches!(self.session, SessionState::Ready(ProtocolVersion::Legacy)) {
                    result
                        .as_object_mut()
                        .expect("tool result is an object")
                        .remove("structuredContent");
                }
                Ok(result)
            }
            _ => Err(RpcError {
                code: -32601,
                message: format!("method not found: {method}"),
            }),
        }
    }

    fn call_tool(&mut self, params: &Value) -> Result<Value, RpcError> {
        let name = params["name"]
            .as_str()
            .ok_or_else(|| RpcError::invalid_params("tool name is required"))?;
        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
        if matches!(
            name,
            "plan_pinax_scan"
                | "discover_pinax_tables"
                | "execute_pinax_scan"
                | "discover_pinax_ontology"
        ) {
            return Err(RpcError::invalid_params(
                "governed scan tools require handle_line_async",
            ));
        }
        if !arguments.is_object() {
            return Err(RpcError::invalid_params("arguments must be an object"));
        }
        if !tool_definitions()
            .as_array()
            .expect("embedded tools are an array")
            .iter()
            .any(|tool| tool["name"] == name)
        {
            return Err(RpcError::invalid_params("unknown tool"));
        }
        let result = serde_json::from_value::<ToolCall>(json!({"name":name,"arguments":arguments}))
            .map_err(|error| RpcError::invalid_params(&error.to_string()))
            .and_then(|request| self.execute_tool(request));
        match result {
            Ok(value) => Ok(json!({
                "content": [{"type": "text", "text": value.to_string()}],
                "structuredContent": value,
                "isError": false,
            })),
            Err(error) => Ok(json!({
                "content": [{"type": "text", "text": error.message}],
                "isError": true,
            })),
        }
    }

    fn execute_tool(&mut self, request: ToolCall) -> Result<Value, RpcError> {
        match request {
            ToolCall::ValidatePinax(proposal) => {
                let arguments = serde_json::to_value(proposal)
                    .map_err(|error| RpcError::internal(&error.to_string()))?;
                pinax::validate_proposal(arguments)
                    .map_err(|error| RpcError::invalid_params(&error.to_string()))
            }
            ToolCall::BuildBundle(arguments) => Ok(AiNavigator.build(arguments.into()).bundle),
            ToolCall::Story(_) => run_qglake_story()
                .and_then(|report| Ok(serde_json::to_value(report)?))
                .map_err(|error| RpcError::internal(&error.to_string())),
            ToolCall::Verify(arguments) => serde_json::to_value(arguments.envelope.verify())
                .map_err(|error| RpcError::internal(&error.to_string())),
            ToolCall::Import(arguments) => {
                let document = match (arguments.osi, arguments.croissant) {
                    (Some(document), None) => document,
                    (None, Some(croissant)) if croissant.is_object() => {
                        OsiDocument::from_croissant_json(&croissant, "qg_lakehouse")
                            .map_err(|error| RpcError::invalid_params(&error.to_string()))?
                    }
                    _ => {
                        return Err(RpcError::invalid_params(
                            "pass exactly one OSI or Croissant object",
                        ));
                    }
                };
                let model = &document.semantic_model;
                if !self.registry.contains_key(&model.name) && self.registry.len() >= 128 {
                    return Err(RpcError::invalid_params("session model limit exceeded"));
                }
                let mut bytes = serde_json::to_vec(&document)
                    .map_err(|error| RpcError::internal(&error.to_string()))?
                    .len();
                for (name, existing) in &self.registry {
                    if name != &model.name {
                        bytes += serde_json::to_vec(existing)
                            .map_err(|error| RpcError::internal(&error.to_string()))?
                            .len();
                    }
                }
                if bytes > 64 * 1024 * 1024 {
                    return Err(RpcError::invalid_params(
                        "session model byte limit exceeded",
                    ));
                }
                let summary = json!({"imported":model.name,"datasets":model.datasets.len(),"metrics":model.metrics.len()});
                self.registry.insert(model.name.clone(), document);
                Ok(summary)
            }
            ToolCall::Search(arguments) => {
                if arguments.term.trim().is_empty() {
                    return Err(RpcError::invalid_params("search term must not be empty"));
                }
                let needle = arguments.term.to_lowercase();
                let matches: Vec<Value> = self
                    .registry
                    .values()
                    .flat_map(|document| search_model(&document.semantic_model, &needle))
                    .collect();
                Ok(json!({"term":needle,"matches":matches}))
            }
            ToolCall::Answer(arguments) => {
                if arguments.question.trim().is_empty() {
                    return Err(RpcError::invalid_params("question must not be empty"));
                }
                let models: Vec<OsiDocument> = self.registry.values().cloned().collect();
                answer_over_models(&models, &arguments.question)
                    .map_err(|error| RpcError::internal(&error.to_string()))
            }
        }
    }
}

enum Incoming {
    Modern(modern::Request),
    Cancelled(Value),
    Request {
        id: Value,
        method: String,
        params: Value,
    },
    Reply(String),
    Notification,
}

fn encode_response(id: Value, result: Result<Value, RpcError>) -> String {
    match result {
        Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}).to_string(),
        Err(error) => rpc_failure(id, error.code, &error.message),
    }
}

fn registry_error_message(error: &crate::registry_service::RegistryServiceError) -> &'static str {
    use crate::registry_service::RegistryServiceError as E;
    use pinax::adapters::PinaxScanError as S;
    match error {
        E::Ontology(_) => "central ontology unavailable or stale",
        E::Authentication => "registry request authentication failed",
        E::Configuration => "registry service configuration is invalid",
        E::IntentLimit | E::Json(_) | E::Contract(_) | E::Scan(S::Contract(_)) => {
            "invalid registry scan intent"
        }
        E::Catalog(lakecat_core::LakeCatError::Forbidden(_))
        | E::Scan(S::Backend(lakecat_core::LakeCatError::Forbidden(_))) => "registry scan denied",
        E::Catalog(lakecat_core::LakeCatError::Conflict(_))
        | E::Scan(S::Backend(lakecat_core::LakeCatError::Conflict(_))) => {
            "registry catalog or snapshot drift"
        }
        E::Catalog(_) | E::Scan(S::Backend(_)) => "registry backend unavailable",
        E::Scan(S::Denied) => "registry scan denied",
        E::Scan(S::Unresolved) => "registry policy decision unresolved",
        E::Scan(S::PolicyMismatch) => "registry policy binding mismatch",
        E::Scan(S::Drift) => "registry catalog or snapshot drift",
    }
}

fn rpc_failure(id: Value, code: i64, message: &str) -> String {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}).to_string()
}

struct RpcError {
    code: i64,
    message: String,
}

impl RpcError {
    fn invalid_params(message: &str) -> Self {
        Self {
            code: -32602,
            message: message.to_string(),
        }
    }

    fn internal(message: &str) -> Self {
        Self {
            code: -32603,
            message: message.to_string(),
        }
    }
}

fn tool_definitions() -> Value {
    serde_json::from_str::<Value>(include_str!("../python/querygraph/mcp_contract.json"))
        .expect("embedded MCP contract is valid JSON")["tools"]
        .clone()
}

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod tests;
