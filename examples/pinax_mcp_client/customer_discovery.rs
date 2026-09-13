//! Metadata-only agent journey. All names and meanings come from signed MCP calls.
use super::{Context, Result, Value, ensure, json, send, signed};

pub(super) async fn run(client: &reqwest::Client, url: &str) -> Result<Value> {
    let mut consultations = Vec::new();
    let mut request_id = 10;
    let mut pin = None;
    for query in ["customer", "account", "user", "enterprise customer id"] {
        let mut cursor = Value::Null;
        let mut pages = Vec::new();
        loop {
            ensure!(pages.len() < 1000, "discovery pagination budget exceeded");
            let intent = json!({"purpose":"customer_discovery", "query":query, "limit":100, "cursor":cursor});
            let request = signed(request_id, "discover_pinax_ontology", intent);
            request_id += 1;
            let response = send(client, url, &request).await?;
            ensure!(
                response["result"]["isError"] == false,
                "semantic discovery failed"
            );
            let value = &response["result"]["structuredContent"];
            ensure!(
                value["search"]["truncated"] == false,
                "refine query: semantic results truncated"
            );
            let digests = (
                value["search"]["ontology_digest"].clone(),
                value["search"]["registry_digest"].clone(),
            );
            if let Some(expected) = &pin {
                ensure!(expected == &digests, "meaning changed during discovery");
            } else {
                pin = Some(digests);
            }
            let next = value["next_cursor"].clone();
            ensure!(
                next.is_null() || next != cursor,
                "discovery cursor did not advance"
            );
            pages.push(response);
            if next.is_null() {
                break;
            }
            cursor = next;
        }
        consultations.push(json!({"query":query,"pages":pages}));
    }
    let customer_pages = consultations[0]["pages"]
        .as_array()
        .context("customer pages")?;
    let mut locations = Vec::new();
    for page in customer_pages {
        let value = &page["result"]["structuredContent"];
        let matches = value["search"]["matches"]
            .as_array()
            .context("semantic matches")?;
        for found in matches
            .iter()
            .filter(|m| m["binding"]["target"]["kind"] == "table")
        {
            let name = &found["binding"]["target"]["table"];
            let dataset = value["exports"]["datasets"]
                .as_array()
                .context("datasets")?
                .iter()
                .find(|d| &d["croissant"]["name"] == name)
                .context("bound dataset")?;
            locations.push(json!({"term":found["concept"]["label"], "definition":found["concept"]["definition"],
                "aliases":found["concept"]["aliases"], "steward":found["concept"]["steward"],
                "sources":found["concept"]["sources"], "concept":found["concept"]["id"],
                "review":found["binding"]["review"], "table":name,
                "table_revision":found["binding"]["table_revision"],
                "fields":dataset["croissant"]["recordSet"][0]["field"]}));
        }
    }
    let (ontology_digest, registry_digest) = pin.context("discovery pin")?;
    Ok(
        json!({"question":"What are customers called where?", "locations":locations,
        "ontology_digest":ontology_digest,"registry_digest":registry_digest,
        "consultations":consultations,"scope":"Authorized registered inventory for customer_discovery; metadata only",
        "interpretation":"Multiple table meanings are candidates, not interchangeable populations. Inspect definitions and shared concept IDs before choosing a table or proposing a join."}),
    )
}
