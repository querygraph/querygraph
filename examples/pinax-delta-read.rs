//! Native Sail Delta reader invoked only by the authenticated demo catalog owner.
use anyhow::{Context, Result, ensure};
use arrow::{
    ipc::reader::StreamReader,
    json::{ArrayWriter, WriterBuilder},
};
use grust::{SailConfig, SailGraphStore};
use lakecat_core::sail::ScanPlanningRequest;
use serde_json::json;
use std::{
    io::{Cursor, Read},
    time::Duration,
};
#[path = "pinax_delta/log.rs"]
mod log;
#[path = "pinax_delta/predicate.rs"]
mod predicate;
#[path = "pinax_delta/sql.rs"]
mod sql;

async fn read(request: ScanPlanningRequest) -> Result<serde_json::Value> {
    ensure!(
        !request.is_incremental_scan(),
        "Incremental Delta reads are not supported by this demo"
    );
    let version = request
        .snapshot_id
        .context("Exact Delta version required")?;
    ensure!(
        request.table_metadata["qg.table-format"] == "delta",
        "Not a Delta catalog record"
    );
    ensure!(
        request.table_metadata["current-snapshot-id"] == version,
        "Delta version differs from catalog"
    );
    let limit = request
        .limit
        .filter(|n| (1..=1000).contains(n))
        .context("Bounded rows required")?;
    ensure!(!request.projection.is_empty(), "Projection required");
    let path = location(&request.table_metadata)?;
    let expected = request.table_metadata["qg.delta-log-sha256"]
        .as_str()
        .context("Log fingerprint")?
        .to_owned();
    let check_path = path.clone();
    ensure!(
        tokio::task::spawn_blocking(move || log::fingerprint(&check_path, version)).await??
            == expected,
        "Delta log drift"
    );
    let projection = request
        .projection
        .iter()
        .map(|c| sql::identifier(c))
        .collect::<Result<Vec<_>>>()?
        .join(", ");
    let filter = predicate::filters(&request.filters)?;
    let predicate = if filter.is_empty() {
        String::new()
    } else {
        format!(" WHERE {filter}")
    };
    let statement = format!(
        "SELECT {projection} FROM delta.`{}` VERSION AS OF {version}{predicate} LIMIT {limit}",
        path.display()
    );
    let sail = SailGraphStore::connect(SailConfig {
        endpoint: "http://127.0.0.1:15051".into(),
        user_id: "pinax-delta-owner".into(),
        batch_size: 128,
        ..SailConfig::default()
    })
    .await?;
    let chunks = sail
        .query_arrow_ipc_bounded(&statement, 32, 2 * 1024 * 1024)
        .await?;
    let mut writer: ArrayWriter<Vec<u8>> = WriterBuilder::new()
        .with_explicit_nulls(true)
        .build(Vec::new());
    for chunk in chunks {
        for batch in StreamReader::try_new(Cursor::new(chunk), None)? {
            writer.write(&batch?)?;
        }
    }
    writer.finish()?;
    let bytes = writer.into_inner();
    ensure!(bytes.len() <= 1024 * 1024, "Delta row output exceeds bound");
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&bytes)?;
    ensure!(rows.len() as u64 <= limit, "Delta row count exceeds bound");
    ensure!(
        tokio::task::spawn_blocking(move || log::fingerprint(&path, version)).await?? == expected,
        "Delta log changed during read"
    );
    Ok(json!({"snapshot_id":version,"columns":request.projection,"rows":rows}))
}
fn main() -> Result<()> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 1024 * 1024, "Request exceeds bound");
    let request = serde_json::from_slice(&bytes)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    let value = runtime
        .block_on(async { tokio::time::timeout(Duration::from_secs(30), read(request)).await })??;
    println!("{}", serde_json::to_string(&value)?);
    Ok(())
}

fn location(metadata: &serde_json::Value) -> Result<std::path::PathBuf> {
    let uri = metadata["location"].as_str().context("Delta location")?;
    let path = uri
        .strip_prefix("file://")
        .context("Only retained local Delta tables are supported")?;
    ensure!(
        path.starts_with('/')
            && path
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b)),
        "Invalid retained Delta path"
    );
    Ok(std::path::Path::new(path).canonicalize()?)
}

#[cfg(test)]
#[path = "pinax_delta/tests.rs"]
mod tests;
