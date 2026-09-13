//! Test driver for Sail's buffered Iceberg read, not an authorization endpoint.
use datafusion::{
    arrow::json::ArrayWriter,
    execution::{memory_pool::GreedyMemoryPool, runtime_env::RuntimeEnvBuilder},
    prelude::{SessionConfig, SessionContext},
};
use sail_iceberg::{
    datasource::bounded_read::{BufferedReadLimits, read_snapshot_buffered},
    datasource::rest_predicate::compile_rest_predicate,
    table::Table,
};
use std::{path::PathBuf, sync::Arc, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(std::env::args().nth(1).ok_or("request required")?);
    let request: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let runtime = RuntimeEnvBuilder::new()
        .with_memory_pool(Arc::new(GreedyMemoryPool::new(64 * 1024 * 1024)))
        .build_arc()?;
    let context = SessionContext::new_with_config_rt(
        SessionConfig::new()
            .with_batch_size(32)
            .with_target_partitions(1),
        runtime,
    );
    let operation = async {
        let table = Table::load_with_metadata_location(
            &context.state(),
            request["location"].as_str().unwrap().parse().map_err(|_| {
                datafusion::common::DataFusionError::Plan("invalid fixture URL".into())
            })?,
            Some(request["metadata"].as_str().unwrap().into()),
        )
        .await?;
        let projection: Vec<String> =
            serde_json::from_value(request["projection"].clone()).unwrap();
        let filters = if let Some(filter) = request.get("filter") {
            vec![compile_rest_predicate(filter)?]
        } else if request["tenant"].is_null() {
            vec![]
        } else {
            vec![compile_rest_predicate(
                &serde_json::json!({"type":"eq","term":"tenant","value":request["tenant"]}),
            )?]
        };
        let batches = read_snapshot_buffered(
            &context,
            &table,
            request["snapshot"].as_i64().unwrap(),
            &projection,
            filters,
            BufferedReadLimits::new(
                request["rows"].as_u64().unwrap() as usize,
                request["bytes"].as_u64().unwrap() as usize,
            )?,
        )
        .await?;
        let mut writer = ArrayWriter::new(Vec::new());
        writer.write_batches(&batches.iter().collect::<Vec<_>>())?;
        writer.finish()?;
        Ok::<_, datafusion::common::DataFusionError>(writer.into_inner())
    };
    match tokio::time::timeout(Duration::from_secs(30), operation).await {
        Ok(Ok(bytes)) => println!("{}", String::from_utf8(bytes)?),
        _ => {
            println!("{{\"error\":\"execution rejected\"}}");
            std::process::exit(1);
        }
    }
    Ok(())
}
