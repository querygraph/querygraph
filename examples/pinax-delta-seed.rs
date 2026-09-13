//! Create an independent retained Delta demo with Sail's native Delta writer.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use grust::{SailConfig, SailGraphStore};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
#[path = "pinax_delta/log.rs"]
mod log;
#[path = "pinax_delta/sql.rs"]
mod sql;

#[derive(Parser)]
struct Arguments {
    #[arg(long)]
    existing: PathBuf,
    #[arg(long)]
    destination: PathBuf,
}
fn load(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
fn save(path: &Path, value: &Value) -> Result<()> {
    Ok(std::fs::write(path, serde_json::to_vec_pretty(value)?)?)
}
async fn prepare(args: Arguments) -> Result<Value> {
    ensure!(
        !args.destination.exists(),
        "Refusing to replace an existing Delta fixture"
    );
    std::fs::create_dir_all(&args.destination)?;
    let dest = args.destination.canonicalize()?;
    for name in [
        "registry.json",
        "policy.yaml",
        "identity.json",
        "recipient.json",
        "agent.seed",
    ] {
        std::fs::copy(args.existing.join(name), dest.join(name))?;
    }
    let sail = SailGraphStore::connect(SailConfig {
        endpoint: "http://127.0.0.1:15051".into(),
        user_id: "pinax-delta-seed".into(),
        ..SailConfig::default()
    })
    .await?;
    let meaning: Value = serde_json::from_str(include_str!("../demo/pinax/customer-meaning.json"))?;
    let seed = load(&args.existing.join("seed.json"))?;
    let mut physical = load(&args.existing.join("customer-tables.json"))?
        .as_array()
        .context("Customer tables")?
        .clone();
    physical.push(json!({"name":"rows", "metadata":seed["metadata"]}));
    let mut tables = Vec::new();
    for original in physical {
        let name = original["name"].as_str().context("Table name")?;
        let path = dest.join("warehouse").join(name);
        ensure!(
            path.to_string_lossy()
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b)),
            "Invalid Delta fixture path"
        );
        let metadata = &original["metadata"];
        let schema = metadata["schemas"]
            .as_array()
            .context("Schemas")?
            .iter()
            .find(|s| s["schema-id"] == metadata["current-schema-id"])
            .context("Current schema")?;
        let fields = schema["fields"].as_array().context("Fields")?;
        let definitions = fields
            .iter()
            .map(|f| {
                let field = sql::identifier(f["name"].as_str().context("Field name")?)?;
                let kind = match f["type"].as_str() {
                    Some("long") => "BIGINT",
                    Some("string") => "STRING",
                    _ => anyhow::bail!("Unsupported demo field type"),
                };
                Ok(format!(
                    "{field} {kind}{}",
                    if f["required"] == true {
                        " NOT NULL"
                    } else {
                        ""
                    }
                ))
            })
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        let table_name = sql::identifier(&format!("pinax_delta_{name}"))?;
        sail.query_arrow_ipc(&format!(
            "CREATE TABLE {table_name} ({definitions}) USING delta LOCATION '{}'",
            path.display()
        ))
        .await?;
        let records = if name == "rows" {
            json!([{"id":1,"tenant":"other","private_email":"hidden"},{"id":2,"tenant":"acme","private_email":"hidden"}])
        } else {
            meaning["records"][name].clone()
        };
        let values = records
            .as_array()
            .context("Records")?
            .iter()
            .map(|row| {
                Ok(format!(
                    "({})",
                    fields
                        .iter()
                        .map(|f| sql::literal(&row[f["name"].as_str().context("Name")?]))
                        .collect::<Result<Vec<_>>>()?
                        .join(", ")
                ))
            })
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        sail.query_arrow_ipc(&format!("INSERT INTO {table_name} VALUES {values}"))
            .await?;
        // CREATE is commit 0; INSERT is commit 1. The catalog's positive snapshot
        // token is the real Delta version, never a fabricated Iceberg snapshot.
        let normalized = json!({"format-version":1,"qg.table-format":"delta", "qg.delta-log-sha256":log::fingerprint(&path,1)?,
            "location":format!("file://{}",path.display()), "current-snapshot-id":1,
            "current-schema-id":0, "schemas":[{"schema-id":0,"type":"struct","fields":fields}],
            "properties":metadata["properties"], "table-uuid":uuid::Uuid::new_v4().to_string()});
        let table = json!({"name":name,"location":normalized["location"],
            "metadata_location":format!("file://{}/_delta_log/00000000000000000001.json",path.display()),"metadata":normalized});
        if name == "rows" {
            save(&dest.join("seed.json"), &table)?;
        } else {
            tables.push(table);
        }
    }
    save(&dest.join("customer-tables.json"), &json!(tables))?;
    let mut config = load(&args.existing.join("registry-service.json"))?;
    config["lakecat_origin"] = json!("http://127.0.0.1:18182/");
    // Share the same reviewed immutable ontology, resolving relative paths at
    // the source config before moving the configuration to the Delta directory.
    if let Some(store) = config["ontology"]["store"].as_str() {
        config["ontology"]["store"] = json!(args.existing.join(store).canonicalize()?);
    }
    save(&dest.join("registry-service.json"), &config)?;
    Ok(json!({"format":"delta", "version":1, "tables":4,"directory":dest}))
}
fn main() -> Result<()> {
    let args = Arguments::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    println!("{}", runtime.block_on(prepare(args))?);
    Ok(())
}
