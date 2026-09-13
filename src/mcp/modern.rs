//! MCP 2026-07-28: each request establishes its own protocol context.
//!
//! Legacy connection state is never consulted. Imported models are immutable,
//! bounded application objects referenced by unguessable, explicit handles.
//! Pinax remains the authority for signed ontology discovery and reads.
use super::*;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const VERSION: &str = "2026-07-28";
pub(super) const VERSION_KEY: &str = "io.modelcontextprotocol/protocolVersion";
const CAPABILITIES_KEY: &str = "io.modelcontextprotocol/clientCapabilities";
const HANDLE_TTL: Duration = Duration::from_secs(3600);
const MAX_MODELS: usize = 128;
const MAX_MODEL_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Default)]
pub(super) struct Server {
    models: Arc<Mutex<ModelStore>>,
    pub(super) registry_service: Option<Arc<crate::registry_service::RegistryService>>,
}

#[derive(Default)]
struct ModelStore {
    entries: BTreeMap<String, StoredModel>,
}

struct StoredModel {
    document: OsiDocument,
    bytes: usize,
    created: Instant,
}

pub(super) struct Request {
    pub id: Value,
    pub method: String,
    pub params: Value,
}

pub(super) struct Error {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl From<RpcError> for Error {
    fn from(error: RpcError) -> Self {
        Self {
            code: error.code,
            message: error.message,
            data: None,
        }
    }
}

impl Error {
    pub(super) fn new(code: i64, message: &str) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub(super) fn response(self, id: Value) -> Value {
        let mut value = json!({"jsonrpc":"2.0","error":{"code":self.code,"message":self.message}});
        if !id.is_null() {
            value["id"] = id;
        }
        if let Some(data) = self.data {
            value["error"]["data"] = data;
        }
        value
    }
}

pub(super) fn recognizes(method: &str, params: &Value) -> bool {
    method == "server/discover"
        || params["_meta"].get(VERSION_KEY).is_some()
        || params["_meta"].get(CAPABILITIES_KEY).is_some()
}

impl Request {
    pub(super) fn parse(id: Value, method: String, params: Value) -> Result<Self, Error> {
        if !(id.is_string() || id.is_i64() || id.is_u64()) {
            return Err(Error::new(-32600, "request ID must be a string or integer"));
        }
        let meta = &params["_meta"];
        let version = meta[VERSION_KEY]
            .as_str()
            .ok_or_else(|| Error::new(-32602, "per-request protocolVersion is required"))?;
        if version != VERSION {
            return Err(Error {
                code: -32022,
                message: "Unsupported protocol version".into(),
                data: Some(json!({"supported":[VERSION],"requested":version})),
            });
        }
        if !meta[CAPABILITIES_KEY].is_object() {
            return Err(Error::new(
                -32602,
                "per-request clientCapabilities must be an object",
            ));
        }
        if let Some(info) = meta.get("io.modelcontextprotocol/clientInfo")
            && (!info["name"].is_string() || !info["version"].is_string())
        {
            return Err(Error::new(-32602, "clientInfo requires name and version"));
        }
        Ok(Self { id, method, params })
    }

    pub(super) fn governed(&self) -> bool {
        self.method == "tools/call" && governed_tool(&self.params)
    }

    pub(super) fn validate_method(&self) -> Result<(), Error> {
        match self.method.as_str() {
            "server/discover" => Ok(()),
            "tools/list" if self.params.get("cursor").is_none() => Ok(()),
            "tools/list" => Err(Error::new(
                -32602,
                "this tool list has no continuation cursor",
            )),
            "tools/call" => {
                let name = self.params["name"]
                    .as_str()
                    .ok_or_else(|| Error::new(-32602, "tool name is required"))?;
                if !self.params.get("arguments").is_none_or(Value::is_object) {
                    return Err(Error::new(-32602, "arguments must be an object"));
                }
                if !tool_definitions()
                    .as_array()
                    .expect("embedded tools")
                    .iter()
                    .any(|tool| tool["name"] == name)
                {
                    return Err(Error::new(-32602, "unknown tool"));
                }
                Ok(())
            }
            _ => Err(Error::new(
                -32601,
                "method not found; HTTP supports MCP 2026-07-28",
            )),
        }
    }
}

pub(super) fn governed_tool(params: &Value) -> bool {
    matches!(
        params["name"].as_str(),
        Some(
            "plan_pinax_scan"
                | "execute_pinax_scan"
                | "discover_pinax_tables"
                | "discover_pinax_ontology"
        )
    )
}

pub(super) fn complete(id: Value, result: Result<Value, Error>) -> Value {
    match result {
        Ok(mut result) => {
            result["resultType"] = json!("complete");
            result["_meta"]["io.modelcontextprotocol/serverInfo"] =
                json!({"name":"querygraph","version":env!("CARGO_PKG_VERSION")});
            json!({"jsonrpc":"2.0","id":id,"result":result})
        }
        Err(error) => error.response(id),
    }
}

impl Server {
    pub(super) async fn dispatch(&self, request: &Request) -> Result<Value, Error> {
        request.validate_method()?;
        if request.governed() {
            self.worker()
                .call_registry_tool(&request.params)
                .await
                .map_err(Into::into)
        } else {
            self.dispatch_sync(request)
        }
    }

    fn worker(&self) -> McpServer {
        McpServer {
            registry: BTreeMap::new(),
            session: SessionState::Ready(ProtocolVersion::Current),
            registry_service: self.registry_service.clone(),
            modern: self.clone(),
        }
    }

    pub(super) fn dispatch_sync(&self, request: &Request) -> Result<Value, Error> {
        request.validate_method()?;
        match request.method.as_str() {
            "server/discover" => Ok(
                json!({"supportedVersions":[VERSION],"capabilities":{"tools":{}},"ttlMs":0,"cacheScope":"public","instructions":"Consult discover_pinax_ontology with a purpose-bound signed intent for reviewed central ontology mappings, Croissant/CDIF and digest evidence. Discovery grants no read authority. Imported models return a model_handle; pass model_handles explicitly to search/answer. Handles expire after one hour and on process restart."}),
            ),
            "tools/list" => Ok(json!({"tools":definitions(),"ttlMs":0,"cacheScope":"public"})),
            "tools/call" => self.call_tool(&request.params),
            _ => Err(Error::new(-32601, "method not found")),
        }
    }

    fn call_tool(&self, params: &Value) -> Result<Value, Error> {
        let mut params = params.clone();
        let name = params["name"].as_str().unwrap_or_default().to_owned();
        let mut worker = self.worker();
        if matches!(
            name.as_str(),
            "search_semantic_model" | "search_semantic_models" | "answer_question"
        ) {
            let handles = params["arguments"]
                .as_object_mut()
                .and_then(|args| args.remove("model_handles"));
            match self.resolve(handles) {
                Ok(models) => worker.registry = models,
                Err(error) => return Ok(tool_error(&error.message)),
            }
        }
        let mut result = worker.call_tool(&params).map_err(Error::from)?;
        if name == "import_semantic_model" && result["isError"] == false {
            let Some((_, document)) = worker.registry.into_iter().next() else {
                return Err(Error::new(-32603, "import did not produce a model"));
            };
            match self.insert(document) {
                Ok(handle) => {
                    result["structuredContent"]["model_handle"] = json!(handle);
                    result["structuredContent"]["expires_in_seconds"] = json!(HANDLE_TTL.as_secs());
                    result["content"][0]["text"] = json!(result["structuredContent"].to_string());
                }
                Err(error) => return Ok(tool_error(&error.message)),
            }
        }
        Ok(result)
    }

    fn insert(&self, document: OsiDocument) -> Result<String, Error> {
        let bytes = serde_json::to_vec(&document)
            .map_err(|_| Error::new(-32603, "model serialization failed"))?
            .len();
        let mut store = self
            .models
            .lock()
            .map_err(|_| Error::new(-32603, "model store unavailable"))?;
        store
            .entries
            .retain(|_, model| model.created.elapsed() < HANDLE_TTL);
        if store.entries.len() >= MAX_MODELS
            || bytes
                + store
                    .entries
                    .values()
                    .map(|model| model.bytes)
                    .sum::<usize>()
                > MAX_MODEL_BYTES
        {
            return Err(Error::new(-32602, "model store capacity exceeded"));
        }
        let handle = uuid::Uuid::new_v4().to_string();
        store.entries.insert(
            handle.clone(),
            StoredModel {
                document,
                bytes,
                created: Instant::now(),
            },
        );
        Ok(handle)
    }

    fn resolve(&self, handles: Option<Value>) -> Result<BTreeMap<String, OsiDocument>, Error> {
        let handles: Vec<String> = serde_json::from_value(handles.unwrap_or(json!([])))
            .map_err(|_| Error::new(-32602, "model_handles must be an array of strings"))?;
        if handles.len() > MAX_MODELS {
            return Err(Error::new(-32602, "too many model handles"));
        }
        let store = self
            .models
            .lock()
            .map_err(|_| Error::new(-32603, "model store unavailable"))?;
        let mut selected = BTreeMap::new();
        for handle in handles {
            let model = store
                .entries
                .get(&handle)
                .filter(|model| model.created.elapsed() < HANDLE_TTL)
                .ok_or_else(|| Error::new(-32602, "unknown or expired model handle"))?;
            if selected
                .insert(
                    model.document.semantic_model.name.clone(),
                    model.document.clone(),
                )
                .is_some()
            {
                return Err(Error::new(-32602, "selected models have duplicate names"));
            }
        }
        Ok(selected)
    }
}

fn tool_error(message: &str) -> Value {
    json!({"content":[{"type":"text","text":message}],"isError":true})
}

fn definitions() -> Value {
    let mut tools = tool_definitions();
    for tool in tools.as_array_mut().expect("embedded tools") {
        match tool["name"].as_str().unwrap_or_default() {
            "search_semantic_model" | "search_semantic_models" | "answer_question" => {
                tool["inputSchema"]["properties"]["model_handles"] = json!({"type":"array","items":{"type":"string"},"maxItems":MAX_MODELS,"description":"Explicit model handles returned by import_semantic_model; omitted means no imported models. Central governed ontology discovery uses discover_pinax_ontology."});
            }
            "import_semantic_model" => {
                tool["description"] = json!(
                    "Import an OSI or Croissant document as an immutable application object. Returns a model_handle valid for one hour or until server restart. Pass it explicitly in model_handles on search/answer calls; possession grants access to the imported document, never to lakehouse rows."
                );
                tool["outputSchema"]["properties"]["model_handle"] = json!({"type":"string"});
                tool["outputSchema"]["properties"]["expires_in_seconds"] =
                    json!({"type":"integer"});
            }
            _ => {}
        }
    }
    tools
}

#[cfg(test)]
#[path = "modern_tests.rs"]
mod tests;
