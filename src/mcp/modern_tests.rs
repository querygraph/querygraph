use super::*;

fn request(method: &str, mut params: Value) -> Value {
    params["_meta"] = json!({VERSION_KEY:VERSION,CAPABILITIES_KEY:{}});
    json!({"jsonrpc":"2.0","id":1,"method":method,"params":params})
}

fn call(server: &mut McpServer, request: Value) -> Value {
    serde_json::from_str(&server.handle_line(&request.to_string()).unwrap()).unwrap()
}

#[test]
fn modern_requests_need_no_handshake_and_never_inherit_metadata() {
    let mut server = McpServer::new();
    let response = call(&mut server, request("server/discover", json!({})));
    assert_eq!(response["result"]["resultType"], "complete");
    assert_eq!(response["result"]["supportedVersions"], json!([VERSION]));
    assert_eq!(response["result"]["ttlMs"], 0);
    assert_eq!(response["result"]["cacheScope"], "public");
    assert_eq!(
        call(&mut server, request("ping", json!({})))["error"]["code"],
        -32601
    );
    let tools = call(&mut server, request("tools/list", json!({})));
    assert!(
        tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "discover_pinax_ontology")
    );
    let mut missing = request("tools/list", json!({}));
    missing["params"]["_meta"]
        .as_object_mut()
        .unwrap()
        .remove(CAPABILITIES_KEY);
    assert_eq!(call(&mut server, missing)["error"]["code"], -32602);
    let mut wrong = request("tools/list", json!({}));
    wrong["params"]["_meta"][VERSION_KEY] = json!("2099-01-01");
    let response = call(&mut server, wrong);
    assert_eq!(response["error"]["code"], -32022);
    assert_eq!(response["error"]["data"]["supported"], json!([VERSION]));
    let mut null = request("ping", json!({}));
    null["id"] = Value::Null;
    let response = call(&mut server, null);
    assert_eq!(response["error"]["code"], -32600);
    assert!(response.get("id").is_none());
    // A modern request does not silently initialize a legacy session.
    assert_eq!(
        call(
            &mut server,
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})
        )["error"]["code"],
        -32002
    );
}

#[test]
fn explicit_handles_isolate_same_named_imports_and_expire() {
    let mut server = McpServer::new();
    let import = |field| {
        request(
            "tools/call",
            json!({"name":"import_semantic_model","arguments":{"croissant":{"name":"Energy","recordSet":[{"field":[{"name":field}]}]}}}),
        )
    };
    let one = call(&mut server, import("monthly_cost"));
    let two = call(&mut server, import("private_income"));
    let handle = one["result"]["structuredContent"]["model_handle"].clone();
    assert!(handle.is_string());
    assert_ne!(handle, two["result"]["structuredContent"]["model_handle"]);
    let search = |handles: Value| {
        request(
            "tools/call",
            json!({"name":"search_semantic_models","arguments":{"term":"cost","model_handles":handles}}),
        )
    };
    assert_eq!(
        call(&mut server, search(json!([])))["result"]["structuredContent"]["matches"],
        json!([])
    );
    let selected = call(&mut server, search(json!([handle])));
    assert!(
        !selected["result"]["structuredContent"]["matches"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!selected.to_string().contains("private_income"));
    assert_eq!(
        call(&mut server, search(json!(["guessed"])))["result"]["isError"],
        true
    );
    server
        .modern
        .models
        .lock()
        .unwrap()
        .entries
        .get_mut(handle.as_str().unwrap())
        .unwrap()
        .created = Instant::now() - HANDLE_TTL;
    assert_eq!(
        call(&mut server, search(json!([handle])))["result"]["isError"],
        true
    );
}

#[tokio::test]
async fn modern_governed_tools_fail_closed_without_host_configuration() {
    let mut server = McpServer::new();
    let message = request(
        "tools/call",
        json!({"name":"discover_pinax_ontology","arguments":{}}),
    );
    let response: Value = serde_json::from_str(
        &server
            .handle_line_async(&message.to_string())
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(response["result"]["resultType"], "complete");
    assert_eq!(response["result"]["isError"], true);
    assert!(response["result"].get("structuredContent").is_none());
}
