use super::*;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

struct PendingCatalog {
    inner: Arc<Catalog>,
    entered: Sender<()>,
    dropped: Sender<()>,
}

struct DropSignal(Sender<()>);
impl Drop for DropSignal {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

#[async_trait]
impl RegistryCatalog for PendingCatalog {
    async fn load_metadata(&self, _: &TableIdent, _: &Principal) -> LakeCatResult<Value> {
        let _guard = DropSignal(self.dropped.clone());
        self.entered.send(()).unwrap();
        std::future::pending().await
    }
    fn engine(&self) -> &dyn SailCatalogEngine {
        self.inner.as_ref()
    }
}

struct Session {
    client: BufReader<UnixStream>,
    entered: Receiver<()>,
    dropped: Receiver<()>,
    server: std::thread::JoinHandle<anyhow::Result<()>>,
}

impl Session {
    fn new() -> Self {
        Self::new_with_handshake(true)
    }
    fn new_with_handshake(initialize: bool) -> Self {
        let (mut service, catalog) = setup(Verdict::Allow, BackendMode::Available, false);
        let (entered, entered_rx) = channel();
        let (dropped, dropped_rx) = channel();
        service.catalog = Arc::new(PendingCatalog {
            inner: catalog,
            entered,
            dropped,
        });
        let (client, server) = UnixStream::pair().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let server = std::thread::spawn(move || {
            let output = Arc::new(Mutex::new(server.try_clone()?));
            crate::mcp::McpServer::new()
                .with_registry_service(Arc::new(service))
                .serve_io(&mut BufReader::new(server), output)
        });
        let mut session = Self {
            client: BufReader::new(client),
            entered: entered_rx,
            dropped: dropped_rx,
            server,
        };
        if initialize {
            session.send(json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{
            "protocolVersion":crate::mcp::MCP_PROTOCOL_VERSION,"capabilities":{},"clientInfo":{"name":"test","version":"1"}}}));
            assert_eq!(session.receive()["id"], 0);
            session.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        }
        session
    }
    fn send(&mut self, message: Value) {
        writeln!(self.client.get_mut(), "{message}").unwrap();
    }
    fn receive(&mut self) -> Value {
        let mut line = String::new();
        assert!(self.client.read_line(&mut line).unwrap() > 0);
        serde_json::from_str(&line).unwrap()
    }
    fn start(&mut self, id: Value) {
        let (intent, envelope) = request("analytics");
        self.send(
            json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{
            "name":"plan_pinax_scan","arguments":{"intent":intent,"envelope":envelope}}}),
        );
        self.entered.recv_timeout(Duration::from_secs(3)).unwrap();
    }
    fn finish(self, remaining: usize) {
        self.client
            .get_ref()
            .shutdown(std::net::Shutdown::Write)
            .unwrap();
        for _ in 0..remaining {
            self.dropped.recv_timeout(Duration::from_secs(3)).unwrap();
        }
        self.server.join().unwrap().unwrap();
    }
}

#[test]
fn cancellation_drops_only_matching_work_and_session_remains_usable() {
    let mut session = Session::new();
    session.start(json!(1));
    session.start(json!("1"));
    session
        .send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":1}}));
    session
        .dropped
        .recv_timeout(Duration::from_secs(3))
        .unwrap();
    assert!(session.dropped.try_recv().is_err());
    session.send(json!({"jsonrpc":"2.0","id":2,"method":"ping"}));
    assert_eq!(session.receive()["id"], 2); // No response for cancelled request.
    session.start(json!(1)); // Reuse cannot detach the replacement request.
    session.finish(2);
}

#[test]
fn malformed_and_unknown_cancellation_leave_work_running() {
    let mut session = Session::new();
    session.start(json!(1));
    for params in [
        json!({"requestId":99}),
        json!({"requestId":1,"reason":42}),
        json!({"requestId":null}),
    ] {
        session.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":params}));
    }
    session.send(json!({"jsonrpc":"2.0","id":2,"method":"ping"}));
    assert_eq!(session.receive()["id"], 2);
    assert!(session.dropped.try_recv().is_err());
    session.finish(1);
}

#[test]
fn overload_is_bounded_and_disconnect_drops_all_pending_work() {
    let mut session = Session::new();
    for id in 1..=16 {
        session.start(json!(id));
    }
    let (intent, envelope) = request("analytics");
    session.send(
        json!({"jsonrpc":"2.0","id":17,"method":"tools/call","params":{
        "name":"plan_pinax_scan","arguments":{"intent":intent,"envelope":envelope}}}),
    );
    let response = session.receive();
    assert_eq!(response["id"], 17);
    assert_eq!(response["error"]["code"], -32000);
    assert!(session.entered.try_recv().is_err());
    session.finish(16);
}

#[test]
fn deadline_drops_backend_work_and_reports_timeout_without_rows() {
    let mut session = Session::new();
    session.start(json!(1));
    session
        .client
        .get_ref()
        .set_read_timeout(Some(Duration::from_secs(35)))
        .unwrap();
    let response = session.receive();
    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["isError"], true);
    assert_eq!(
        response["result"]["content"][0]["text"],
        "governed request timed out"
    );
    assert!(response["result"].get("structuredContent").is_none());
    session
        .dropped
        .recv_timeout(Duration::from_secs(3))
        .unwrap();
    session.send(json!({"jsonrpc":"2.0","id":2,"method":"ping"}));
    assert_eq!(session.receive()["id"], 2);
    session.finish(0);
}

#[tokio::test]
async fn stateless_http_disconnect_drops_governed_future() {
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let (mut service, catalog) = setup(Verdict::Allow, BackendMode::Available, false);
    let (entered, entered_rx) = channel();
    let (dropped, dropped_rx) = channel();
    service.catalog = Arc::new(PendingCatalog {
        inner: catalog,
        entered,
        dropped,
    });
    let router = crate::mcp::McpServer::new()
        .with_registry_service(Arc::new(service))
        .http_router(vec![]);
    let (intent, envelope) = request("analytics");
    let message = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}},
        "name":"plan_pinax_scan","arguments":{"intent":intent,"envelope":envelope}}});
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2026-07-28")
        .header("mcp-method", "tools/call")
        .header("mcp-name", "plan_pinax_scan")
        .body(axum::body::Body::from(message.to_string()))
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let mut body = response.into_body();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), body.frame())
            .await
            .is_err()
    );
    entered_rx.try_recv().unwrap();
    assert!(dropped_rx.try_recv().is_err());
    drop(body);
    dropped_rx
        .try_recv()
        .expect("disconnect drops backend work immediately");
}

#[test]
fn modern_stdio_cancellation_works_without_initialization_context() {
    let mut session = Session::new_with_handshake(false);
    let (intent, envelope) = request("analytics");
    session.send(json!({"jsonrpc":"2.0","id":"modern","method":"tools/call","params":{
        "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}},
        "name":"plan_pinax_scan","arguments":{"intent":intent,"envelope":envelope}}}));
    session
        .entered
        .recv_timeout(Duration::from_secs(3))
        .unwrap();
    session.send(
        json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"modern"}}),
    );
    session
        .dropped
        .recv_timeout(Duration::from_secs(3))
        .unwrap();
    session.send(json!({"jsonrpc":"2.0","id":2,"method":"server/discover","params":{
        "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}));
    assert_eq!(session.receive()["result"]["resultType"], "complete");
    session.finish(0);
}
