//! Executable MCP 2026-07-28 acceptance client using only the public demo identity.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use querygraph::{agent::PyTypeDidEnvelope, mcp::MCP_STATELESS_PROTOCOL_VERSION};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::time::Duration;

#[derive(Parser)]
struct Arguments {
    #[arg(long, default_value = "http://127.0.0.1:18082/mcp")]
    url: String,
}

fn message(id: usize, method: &str, mut params: Value) -> Value {
    params["_meta"] = json!({
        "io.modelcontextprotocol/protocolVersion":MCP_STATELESS_PROTOCOL_VERSION,
        "io.modelcontextprotocol/clientCapabilities":{},
        "io.modelcontextprotocol/clientInfo":{"name":"querygraph-pinax-acceptance","version":"1"}
    });
    json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
}

fn signed(id: usize, name: &str, intent: Value) -> Value {
    let intent = intent.to_string();
    let seed = format!("typesec-ed25519-signing\0{}", "\u{7}".repeat(32));
    let envelope = PyTypeDidEnvelope::signed(
        &seed,
        "did:example:registry",
        "invoke",
        &format!("/mcp/{name}"),
        json!({"bodySha256":format!("{:x}",Sha256::digest(intent.as_bytes()))}),
    );
    message(
        id,
        "tools/call",
        json!({"name":name,"arguments":{"intent":intent,"envelope":envelope}}),
    )
}

async fn send(client: &reqwest::Client, url: &str, message: &Value) -> Result<Value> {
    let mut request = client
        .post(url)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", MCP_STATELESS_PROTOCOL_VERSION)
        .header("mcp-method", message["method"].as_str().context("method")?);
    if let Some(name) = message["params"]["name"].as_str() {
        request = request.header("mcp-name", name);
    }
    let response = request.json(message).send().await?.error_for_status()?;
    ensure!(
        !response.headers().contains_key("mcp-session-id"),
        "server minted a session"
    );
    let sse = response
        .headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .is_some_and(|h| h.starts_with("text/event-stream"));
    let text = response.text().await?;
    let response: Value = if sse {
        let data = text
            .lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .collect::<Vec<_>>();
        ensure!(
            data.len() == 1,
            "expected one final response on this stream"
        );
        serde_json::from_str(data[0])?
    } else {
        serde_json::from_str(&text)?
    };
    ensure!(response["id"] == message["id"], "response ID mismatch");
    ensure!(
        response["result"]["resultType"] == "complete",
        "missing complete result: {response}"
    );
    Ok(response)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Arguments::parse();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(35))
        .build()?;
    let discovery = send(
        &client,
        &args.url,
        &message(1, "server/discover", json!({})),
    )
    .await?;
    ensure!(
        discovery["result"]["supportedVersions"] == json!([MCP_STATELESS_PROTOCOL_VERSION]),
        "wrong protocol"
    );
    let tools = send(&client, &args.url, &message(2, "tools/list", json!({}))).await?;
    ensure!(
        tools["result"]["tools"]
            .as_array()
            .context("tools")?
            .iter()
            .any(|tool| tool["name"] == "discover_pinax_ontology"),
        "missing ontology tool"
    );
    let ontology_request = signed(
        3,
        "discover_pinax_ontology",
        json!({"purpose":"analytics","query":"account number","limit":10}),
    );
    let ontology = send(&client, &args.url, &ontology_request).await?;
    ensure!(ontology["result"]["isError"] == false, "ontology failed");
    let semantic = &ontology["result"]["structuredContent"];
    ensure!(
        !semantic.to_string().contains("private_email")
            && !semantic.to_string().contains("row.tenant"),
        "private metadata escaped"
    );
    let matches = semantic["search"]["matches"]
        .as_array()
        .context("matches")?;
    ensure!(
        matches.len() == 1,
        "alias did not identify exactly one field"
    );
    let target = &matches[0]["binding"]["target"];
    let dataset = semantic["exports"]["datasets"]
        .as_array()
        .context("datasets")?
        .iter()
        .find(|dataset| dataset["croissant"]["name"] == target["table"])
        .context("authorized table")?;
    let field = dataset["croissant"]["recordSet"][0]["field"]
        .as_array()
        .context("fields")?
        .iter()
        .find(|field| field["qg:fieldId"] == target["field_id"])
        .context("authorized field")?;
    let execute_request = signed(
        4,
        "execute_pinax_scan",
        json!({"purpose":"analytics","table":target["table"],"columns":[field["name"]],"limit":10}),
    );
    let execution = send(&client, &args.url, &execute_request).await?;
    ensure!(
        execution["result"]["structuredContent"]["rows"] == json!([{"id":2}]),
        "unexpected governed rows"
    );
    ensure!(
        execution["result"]["structuredContent"]["semantic_exports"]["ontology_digest"]
            == semantic["search"]["ontology_digest"],
        "ontology pin mismatch"
    );
    let denial = send(
        &client,
        &args.url,
        &signed(
            5,
            "discover_pinax_ontology",
            json!({"purpose":"marketing","query":"account number","limit":10}),
        ),
    )
    .await?;
    ensure!(
        denial["result"]["structuredContent"]["search"]["matches"] == json!([]),
        "purpose denial disclosed mappings"
    );
    let mut wrong_signature = ontology_request.clone();
    wrong_signature["id"] = json!(6);
    wrong_signature["params"]["arguments"]["intent"] =
        json!("{\"purpose\":\"marketing\",\"query\":\"account number\",\"limit\":10}");
    let wrong_signature = send(&client, &args.url, &wrong_signature).await?;
    ensure!(
        wrong_signature["result"]["isError"] == true
            && wrong_signature["result"].get("structuredContent").is_none(),
        "changed intent accepted"
    );
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"protocol":MCP_STATELESS_PROTOCOL_VERSION,"transport":"stateless-streamable-http","discovery":discovery,"tools":tools,"ontology":ontology,"execution":execution,"purpose_denial":denial,"signature_rejection":wrong_signature,"requests":[ontology_request,execute_request]})
        )?
    );
    Ok(())
}
