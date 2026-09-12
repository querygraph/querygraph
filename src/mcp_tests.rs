use super::*;

fn call(server: &mut McpServer, message: Value) -> Value {
    let response = server
        .handle_line(&message.to_string())
        .expect("request gets a response");
    serde_json::from_str(&response).expect("response is JSON")
}

#[test]
fn handshake_lists_tools_and_answers_over_imported_models() {
    let mut server = McpServer::new();

    let init = call(
        &mut server,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": MCP_PROTOCOL_VERSION, "capabilities": {}}}),
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
    assert!(names.contains(&"validate_fihrist"));

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
    let mut server = McpServer::new();
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
    assert_eq!(unknown_tool["result"]["isError"], true);
}

#[test]
fn fihrist_tool_validates_drafts_without_publishing() {
    let mut server = McpServer::new();
    let document = serde_json::json!({"version": "fihrist.v1", "enterprise": "acme", "tables": []});
    let response = call(
        &mut server,
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"validate_fihrist","arguments":{"document":document.to_string()}}}),
    );
    assert_eq!(response["result"]["structuredContent"]["valid"], true);
    assert_eq!(
        response["result"]["structuredContent"],
        fihrist::validate_proposal(json!({"document": document.to_string()}))
            .expect("standalone Fihrist accepts the draft")
    );
    assert!(server.registry.is_empty());
    let response = call(
        &mut server,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"validate_fihrist","arguments":{"document":"{}"}}}),
    );
    assert_eq!(response["result"]["isError"], true);
}

#[test]
fn fihrist_tool_checks_predecessors_without_importing_them() {
    let mut server = McpServer::new();
    let previous = json!({"version":"fihrist.v1","enterprise":"acme","tables":[]});
    let next = json!({"version":"fihrist.v1","enterprise":"other","tables":[]});
    for (document, rejected) in [(&previous, false), (&next, true)] {
        let response = call(
            &mut server,
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"validate_fihrist","arguments":{
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
