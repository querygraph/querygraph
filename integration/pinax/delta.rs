//! Read-only Delta bridge for the retained demo; LakeCat retains all policy gates.
use async_trait::async_trait;
use lakecat_core::{LakeCatError, LakeCatResult, sail::*};
use serde::Deserialize;
use serde_json::json;
use std::{path::PathBuf, process::Stdio, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

pub struct DeltaDemoEngine {
    pub inner: Arc<dyn SailCatalogEngine>,
    pub reader: PathBuf,
}
fn delta(metadata: &serde_json::Value) -> bool {
    metadata["qg.table-format"] == "delta"
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rows {
    snapshot_id: i64,
    columns: Vec<String>,
    rows: Vec<serde_json::Value>,
}
#[async_trait]
impl SailCatalogEngine for DeltaDemoEngine {
    async fn execute_scan(&self, request: ScanPlanningRequest) -> LakeCatResult<ExecutedScan> {
        if !delta(&request.table_metadata) {
            return self.inner.execute_scan(request).await;
        }
        let payload = serde_json::to_vec(&request)
            .map_err(|_| LakeCatError::Internal("Cannot encode Delta request".into()))?;
        let operation = async {
            let mut child = Command::new(&self.reader)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()?;
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| std::io::Error::other("Missing input"))?;
            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| std::io::Error::other("Missing output"))?;
            let stderr = child
                .stderr
                .take()
                .ok_or_else(|| std::io::Error::other("Missing diagnostics"))?;
            let mut stdout = stdout.take(2 * 1024 * 1024 + 1);
            let mut stderr = stderr.take(64 * 1024 + 1);
            let mut out = Vec::new();
            let mut err = Vec::new();
            let (status, _, _, _) = tokio::try_join!(
                child.wait(),
                async move {
                    stdin.write_all(&payload).await?;
                    stdin.shutdown().await
                },
                stdout.read_to_end(&mut out),
                stderr.read_to_end(&mut err)
            )?;
            if !status.success() || out.len() > 2 * 1024 * 1024 {
                return Err(std::io::Error::other(format!(
                    "Sail Delta read failed: {}",
                    String::from_utf8_lossy(&err)
                )));
            }
            Ok::<_, std::io::Error>(out)
        };
        let bytes = tokio::time::timeout(std::time::Duration::from_secs(35), operation)
            .await
            .map_err(|_| LakeCatError::Internal("Delta read timed out".into()))?
            .map_err(|e| LakeCatError::Internal(e.to_string()))?;
        let result: Rows = serde_json::from_slice(&bytes)
            .map_err(|_| LakeCatError::Internal("Invalid Delta reader response".into()))?;
        Ok(ExecutedScan {
            snapshot_id: result.snapshot_id,
            columns: result.columns,
            rows: result.rows,
        })
    }
    async fn prepare_commit(&self, request: CommitPreparationRequest) -> LakeCatResult<CommitPlan> {
        if delta(&request.current_metadata) || request.new_metadata.as_ref().is_some_and(delta) {
            return Err(LakeCatError::NotSupported(
                "The retained Delta demo is read-only".into(),
            ));
        }
        self.inner.prepare_commit(request).await
    }
    async fn plan_scan(&self, request: ScanPlanningRequest) -> LakeCatResult<ScanPlan> {
        if !delta(&request.table_metadata) {
            return self.inner.plan_scan(request).await;
        }
        let version = request.table_metadata["current-snapshot-id"]
            .as_i64()
            .ok_or_else(|| LakeCatError::Conflict("Missing Delta version".into()))?;
        if request.snapshot_id != Some(version)
            || request.is_incremental_scan()
            || request.projection.is_empty()
        {
            return Err(LakeCatError::Conflict(
                "Delta scan requires the registered version and projection".into(),
            ));
        }
        Ok(ScanPlan {
            planned_by: "lakecat-delta-demo".into(),
            snapshot_id: Some(version),
            scan_tasks: vec![
                json!({"format":"delta","version":version,"execution":"catalog-owner"}),
            ],
            residual_filter: None,
        })
    }
    async fn fetch_scan_tasks(
        &self,
        request: FetchScanTasksRequest,
    ) -> LakeCatResult<FetchScanTasksPlan> {
        if delta(&request.table_metadata) {
            return Err(LakeCatError::NotSupported(
                "Delta demo reads remain inside the catalog owner".into(),
            ));
        }
        self.inner.fetch_scan_tasks(request).await
    }
}
