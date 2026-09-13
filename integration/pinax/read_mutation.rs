//! Fixture-only deterministic mutation after real Sail rows, before release.
use async_trait::async_trait;
use lakecat_core::{LakeCatResult, sail::*};
use lakecat_store::{CatalogStore, PolicyBinding, TableCommit};
use serde::Deserialize;
use serde_json::json;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadMutation {
    PurposeRevoked,
    RegistryDrift,
    SnapshotDrift,
}

pub struct MutatingReader {
    pub inner: Arc<dyn SailCatalogEngine>,
    pub store: Arc<dyn CatalogStore>,
    pub mutation: ReadMutation,
    pub marker: PathBuf,
    pub successor_metadata: serde_json::Value,
    pub successor_location: String,
}

#[async_trait]
impl SailCatalogEngine for MutatingReader {
    async fn execute_scan(&self, request: ScanPlanningRequest) -> LakeCatResult<ExecutedScan> {
        let rows = self.inner.execute_scan(request.clone()).await?;
        // This fixture must prove a nonempty real buffer existed before mutation.
        if rows.rows.is_empty() {
            return Err(lakecat_core::LakeCatError::Internal(
                "fixture expected real rows".into(),
            ));
        }
        match self.mutation {
            ReadMutation::PurposeRevoked => {
                self.store.upsert_policy_binding(PolicyBinding::new(
                    "fixture-rows", request.table.warehouse.clone(),
                    Some(request.table.namespace.clone()), Some(request.table.name.clone()), true,
                    json!({"lakecat:read-restriction":{"allowed-columns":["id"],"row-predicate":{"type":"eq","term":"tenant","value":"acme"}},
                        "permission":[{"action":"read","constraint":[{"leftOperand":"purpose","operator":"eq","rightOperand":"marketing"}]}]}),
                )?).await?;
            }
            ReadMutation::RegistryDrift | ReadMutation::SnapshotDrift => {
                let current = self.store.load_table(&request.table).await?;
                let mut metadata = current.metadata.clone();
                match self.mutation {
                    ReadMutation::RegistryDrift => {
                        metadata["properties"]["pinax.registry-digest"] =
                            json!("changed-by-fixture")
                    }
                    ReadMutation::SnapshotDrift => metadata = self.successor_metadata.clone(),
                    ReadMutation::PurposeRevoked => unreachable!(),
                }
                self.store
                    .commit_table(
                        &request.table,
                        TableCommit {
                            requirements: vec![],
                            updates: vec![],
                            expected_previous_metadata_location: current.metadata_location.clone(),
                            new_metadata_location: match self.mutation {
                                ReadMutation::SnapshotDrift => {
                                    Some(self.successor_location.clone())
                                }
                                _ => current.metadata_location,
                            },
                            new_metadata: Some(metadata),
                            idempotency_key: None,
                            idempotency_request_hash: None,
                            principal: request.principal,
                            authorization_receipt: None,
                        },
                    )
                    .await?;
            }
        }
        std::fs::write(&self.marker, rows.rows.len().to_string())
            .map_err(|_| lakecat_core::LakeCatError::Internal("fixture marker failed".into()))?;
        Ok(rows)
    }

    async fn prepare_commit(&self, request: CommitPreparationRequest) -> LakeCatResult<CommitPlan> {
        self.inner.prepare_commit(request).await
    }
    async fn plan_scan(&self, request: ScanPlanningRequest) -> LakeCatResult<ScanPlan> {
        self.inner.plan_scan(request).await
    }
    async fn fetch_scan_tasks(
        &self,
        request: FetchScanTasksRequest,
    ) -> LakeCatResult<FetchScanTasksPlan> {
        self.inner.fetch_scan_tasks(request).await
    }
}
