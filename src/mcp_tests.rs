use super::*;

#[test]
fn invalid_tool_arguments_are_consistent_tool_failures() {
    let mut server = ready_server();
    for (name, arguments) in [
        ("search_semantic_models", json!({})),
        ("search_semantic_models", json!({"term":""})),
        (
            "search_semantic_models",
            json!({"term":"energy","extra":true}),
        ),
        ("answer_question", json!({"question":42})),
        ("build_navigator_bundle", json!({})),
        ("verify_envelope", json!({"envelope":{}})),
        ("run_qglake_story", json!({"extra":true})),
        (
            "import_semantic_model",
            json!({"osi":{"semantic_model":{"name":"a"}},"croissant":{}}),
        ),
    ] {
        let response = call(
            &mut server,
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":name,"arguments":arguments}}),
        );
        assert_eq!(response["result"]["isError"], true, "{name}: {response}");
        assert!(response.get("error").is_none());
    }
    assert!(server.registry.is_empty());
}

#[test]
fn legacy_clients_receive_only_legacy_result_fields() {
    let mut server = McpServer::new();
    let initialized = call(
        &mut server,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize",
        "params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"legacy","version":"1"}}}),
    );
    assert_eq!(initialized["result"]["protocolVersion"], "2024-11-05");
    server.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    let response = call(
        &mut server,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"search_semantic_models","arguments":{"term":"energy"}}}),
    );
    assert!(response["result"].get("structuredContent").is_none());
    assert_eq!(response["result"]["isError"], false);
    let listed = call(
        &mut server,
        json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}),
    );
    for tool in listed["result"]["tools"].as_array().expect("tool list") {
        assert!(tool.get("outputSchema").is_none());
        assert!(tool.get("annotations").is_none());
    }
}

#[test]
fn oversized_stdio_input_is_bounded_and_closes_the_session() {
    let mut server = McpServer::new();
    let mut input = std::io::Cursor::new(vec![b' '; MAX_MCP_MESSAGE_BYTES + 100]);
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    assert!(server.serve_io(&mut input, output.clone()).is_err());
    assert_eq!(input.position(), (MAX_MCP_MESSAGE_BYTES + 1) as u64);
    let response: Value =
        serde_json::from_slice(&output.lock().unwrap()).expect("bounded error response");
    assert_eq!(response["error"]["code"], -32600);
}

fn call(server: &mut McpServer, message: Value) -> Value {
    let response = server
        .handle_line(&message.to_string())
        .expect("request gets a response");
    serde_json::from_str(&response).expect("response is JSON")
}

fn ready_server() -> McpServer {
    let mut server = McpServer::new();
    let response = call(
        &mut server,
        json!({"jsonrpc":"2.0","id":0,"method":"initialize",
        "params":{"protocolVersion":MCP_PROTOCOL_VERSION,"capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
    );
    assert!(response.get("error").is_none());
    assert!(
        server
            .handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none()
    );
    server
}

#[test]
fn handshake_lists_tools_and_answers_over_imported_models() {
    let mut server = McpServer::new();

    let init = call(
        &mut server,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": MCP_PROTOCOL_VERSION, "capabilities": {}, "clientInfo":{"name":"test","version":"1"}}}),
    );
    assert_eq!(init["result"]["protocolVersion"], MCP_PROTOCOL_VERSION);
    assert_eq!(init["result"]["serverInfo"]["name"], "querygraph");

    // The initialized notification gets no response.
    assert!(
        server
            .handle_line(
                &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string()
            )
            .is_none()
    );

    let tools = call(
        &mut server,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    );
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"answer_question"));
    assert!(names.contains(&"import_semantic_model"));
    assert!(names.contains(&"validate_pinax"));

    let imported = call(
        &mut server,
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
        "params": {"name": "import_semantic_model", "arguments": {"croissant": {
            "name": "Energy Burden",
            "recordSet": [{"field": [{"name": "monthly_cost"}]}],
        }}}}),
    );
    assert_eq!(
        imported["result"]["structuredContent"]["imported"],
        "energy_burden_semantic_model"
    );

    let answer = call(
        &mut server,
        json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call",
               "params": {"name": "answer_question",
                          "arguments": {"question": "what drives monthly energy cost?"}}}),
    );
    let structured = &answer["result"]["structuredContent"];
    assert_eq!(structured["plans"][0]["dataset"], "energy_burden");
    assert!(
        !structured["envelope"]["signature"]
            .as_str()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn unknown_method_and_unknown_tool_are_reported() {
    let mut server = ready_server();
    let missing = call(
        &mut server,
        json!({"jsonrpc": "2.0", "id": 1, "method": "resources/list"}),
    );
    assert_eq!(missing["error"]["code"], -32601);

    let unknown_tool = call(
        &mut server,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
               "params": {"name": "no_such_tool", "arguments": {}}}),
    );
    assert_eq!(unknown_tool["error"]["code"], -32602);
}

#[test]
fn pinax_tool_validates_drafts_without_publishing() {
    let mut server = ready_server();
    let document = serde_json::json!({"version": "pinax.v1", "enterprise": "acme", "tables": []});
    let response = call(
        &mut server,
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"validate_pinax","arguments":{"document":document.to_string()}}}),
    );
    assert_eq!(response["result"]["structuredContent"]["valid"], true);
    assert_eq!(
        response["result"]["structuredContent"],
        pinax::validate_proposal(json!({"document": document.to_string()}))
            .expect("standalone Pinax accepts the draft")
    );
    assert!(server.registry.is_empty());
    let response = call(
        &mut server,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"validate_pinax","arguments":{"document":"{}"}}}),
    );
    assert_eq!(response["result"]["isError"], true);
}

#[test]
fn pinax_tool_checks_predecessors_without_importing_them() {
    let mut server = ready_server();
    let previous = json!({"version":"pinax.v1","enterprise":"acme","tables":[]});
    let next = json!({"version":"pinax.v1","enterprise":"other","tables":[]});
    for (document, rejected) in [(&previous, false), (&next, true)] {
        let response = call(
            &mut server,
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"validate_pinax","arguments":{
                "document":document.to_string(),"previous":previous.to_string()
            }}}),
        );
        if rejected {
            assert_eq!(response["result"]["isError"], true);
        } else {
            assert_eq!(response["result"]["structuredContent"]["valid"], true);
        }
        assert!(server.registry.is_empty());
    }
}

#[test]
fn tools_require_the_completed_handshake() {
    let mut server = McpServer::new();
    let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/list"});
    assert_eq!(call(&mut server, request.clone())["error"]["code"], -32002);
    call(
        &mut server,
        json!({"jsonrpc":"2.0","id":2,"method":"initialize",
        "params":{"protocolVersion":"future-version","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
    );
    assert_eq!(call(&mut server, request.clone())["error"]["code"], -32002);
    server.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    assert!(call(&mut server, request)["result"]["tools"].is_array());
}

#[test]
fn malformed_requests_are_not_silently_treated_as_notifications() {
    let mut server = ready_server();
    for request in [
        json!([]),
        json!(42),
        json!({"method":"ping"}),
        json!({"jsonrpc":"2.0","id":{},"method":"ping"}),
        json!({"jsonrpc":"2.0","id":1,"method":"ping","params":[]}),
    ] {
        assert_eq!(call(&mut server, request)["error"]["code"], -32600);
    }
    assert_eq!(
        call(
            &mut server,
            json!({"jsonrpc":"2.0","id":null,"method":"ping"})
        )["result"],
        json!({})
    );
}
