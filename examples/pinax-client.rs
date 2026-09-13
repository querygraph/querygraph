//! Run signed operations against the isolated Pinax demonstration owner.
//! This example deliberately uses the fixture identity, never production keys.
use anyhow::Result;
use clap::{Parser, ValueEnum};
use querygraph::{agent::PyTypeDidEnvelope, registry_service::config::load_registry_service};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Duration};

#[derive(Parser)]
struct Arguments {
    #[arg(long)]
    config: PathBuf,
    #[arg(value_enum)]
    operation: Operation,
    #[arg(long, default_value = "customer identifier")]
    query: String,
}

#[derive(Clone, Copy, ValueEnum)]
enum Operation {
    Ontology,
    Discover,
    Plan,
    Execute,
    DenyColumn,
    DenyPurpose,
}

impl Operation {
    fn request(self) -> (&'static str, Value) {
        match self {
            Self::Ontology => (
                "/mcp/discover_pinax_ontology",
                json!({"purpose":"analytics","query":"customer identifier","limit":100}),
            ),
            Self::Discover => (
                "/mcp/discover_pinax_tables",
                json!({"purpose":"analytics", "limit":100}),
            ),
            Self::Plan => (
                "/mcp/plan_pinax_scan",
                json!({"table":"rows", "columns":["id"], "purpose":"analytics", "limit":10}),
            ),
            Self::Execute => (
                "/mcp/execute_pinax_scan",
                json!({"table":"rows", "columns":["id"], "purpose":"analytics", "limit":10}),
            ),
            Self::DenyColumn => (
                "/mcp/execute_pinax_scan",
                json!({"table":"rows", "columns":["private_email"], "purpose":"analytics", "limit":10}),
            ),
            Self::DenyPurpose => (
                "/mcp/execute_pinax_scan",
                json!({"table":"rows", "columns":["id"], "purpose":"marketing", "limit":10}),
            ),
        }
    }
}

fn main() -> Result<()> {
    let args = Arguments::parse();
    // Loading can perform the configured activation check before this runtime.
    let service = load_registry_service(&args.config)?;
    let (resource, mut request) = args.operation.request();
    if matches!(args.operation, Operation::Ontology) {
        request["query"] = json!(args.query);
    }
    let mut intent = serde_json::to_string(&request)?;
    // Match TypeSec's domain-separated derivation for the fixture's [7; 32].
    // PyTypeDidEnvelope::signed hashes its seed string once before signing.
    let fixture_seed = format!("typesec-ed25519-signing\0{}", "\u{7}".repeat(32));
    let mut envelope = PyTypeDidEnvelope::signed(
        &fixture_seed,
        "did:example:registry",
        "invoke",
        resource,
        json!({"bodySha256":format!("{:x}", Sha256::digest(intent.as_bytes()))}),
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    let result = runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(30), async {
            let consultation = if matches!(args.operation, Operation::Plan | Operation::Execute) {
                let query =
                    json!({"purpose":"analytics","query":"customer identifier","limit":100})
                        .to_string();
                let signature = PyTypeDidEnvelope::signed(
                    &fixture_seed,
                    "did:example:registry",
                    "invoke",
                    "/mcp/discover_pinax_ontology",
                    json!({"bodySha256":format!("{:x}",Sha256::digest(query.as_bytes()))}),
                );
                let result = service.discover_ontology(&query, &signature).await?;
                let matches = result["search"]["matches"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Missing ontology matches"))?;
                let fields: Vec<_> = matches
                    .iter()
                    .filter(|m| m["binding"]["target"]["kind"] == "field")
                    .collect();
                anyhow::ensure!(
                    fields.len() == 1,
                    "Ontology must resolve exactly one customer identifier field"
                );
                let target = &fields[0]["binding"]["target"];
                let table = target["table"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing bound table"))?;
                let datasets = result["exports"]["datasets"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Missing authorized datasets"))?;
                let dataset = datasets
                    .iter()
                    .find(|d| d["croissant"]["name"] == table)
                    .ok_or_else(|| anyhow::anyhow!("Bound table is not authorized"))?;
                let columns = dataset["croissant"]["recordSet"][0]["field"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Missing authorized fields"))?;
                let column = columns
                    .iter()
                    .find(|c| c["qg:fieldId"] == target["field_id"])
                    .ok_or_else(|| anyhow::anyhow!("Bound field is not authorized"))?;
                request["table"] = json!(table);
                request["columns"] = json!([column["name"]]);
                intent = serde_json::to_string(&request)?;
                envelope = PyTypeDidEnvelope::signed(
                    &fixture_seed,
                    "did:example:registry",
                    "invoke",
                    resource,
                    json!({"bodySha256":format!("{:x}",Sha256::digest(intent.as_bytes()))}),
                );
                Some(result)
            } else {
                None
            };
            let mut output = match args.operation {
                Operation::Ontology => service
                    .discover_ontology(&intent, &envelope)
                    .await
                    .map_err(anyhow::Error::from),
                Operation::Discover => {
                    serde_json::to_value(service.discover(&intent, &envelope).await?)
                        .map_err(anyhow::Error::from)
                }
                Operation::Plan => {
                    serde_json::to_value(service.plan_scan(&intent, &envelope).await?.summary())
                        .map_err(anyhow::Error::from)
                }
                Operation::Execute | Operation::DenyColumn | Operation::DenyPurpose => {
                    serde_json::to_value(service.execute_scan(&intent, &envelope).await?)
                        .map_err(anyhow::Error::from)
                }
            }?;
            if let Some(consultation) = consultation {
                output["ontology_consultation"] = consultation;
            }
            Ok::<Value, anyhow::Error>(output)
        })
        .await
    })??;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
