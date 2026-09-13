//! Preflight all tables, then submit each conditional transition once.

use lakecat_core::{LakeCatError, Principal, TableIdent, content_hash_json};
use serde_json::{Value, json};

use super::catalog::{RegistryCatalogCommit, RegistryDeploymentCatalog};
use super::{
    RegistryDeploymentPlan, RegistryReconciliation, RegistryReconciliationError, TableDeployment,
    TableTransition,
};

/// Deployment errors distinguish preflight rejection from a write that must be
/// reconciled. No error path retries a mutation or claims global readiness.
#[derive(Debug, thiserror::Error)]
pub enum RegistryApplyError {
    /// A catalog read or request preparation failed before any mutation.
    #[error("registry deployment preflight failed")]
    Preflight(#[source] LakeCatError),
    /// Current state matches neither reviewed endpoint, or an old table vanished.
    #[error("registry deployment encountered drift in {0:?}")]
    Drift(TableIdent),
    /// The backend cannot guard an update against a concurrent state change.
    #[error("catalog does not support conditional registry updates")]
    ConditionalCommitUnavailable,
    /// A mutation failed or its result is uncertain; reload before another attempt.
    #[error("registry mutation requires reconciliation for {table:?}")]
    RequiresReconciliation {
        /// The logical table whose write must be inspected.
        table: TableIdent,
        /// Retained for trusted diagnostics; transports must redact backend text.
        #[source]
        source: LakeCatError,
    },
    /// Final readback failed after writes may already have committed.
    #[error("registry deployment requires final reconciliation")]
    Verification(#[source] RegistryReconciliationError),
}

enum PreparedMutation<'a> {
    Create {
        table: &'a TableIdent,
        request: &'a pinax::adapters::CatalogCreatePlan,
    },
    Update {
        table: &'a TableDeployment,
        token: String,
        request: RegistryCatalogCommit,
        key: String,
    },
}

impl RegistryDeploymentPlan {
    /// Digest that an operator must review before executing this immutable plan.
    pub fn target_digest(&self) -> &str {
        &self.target_digest
    }

    /// Apply a reviewed transition with complete preflight and owner-side CAS.
    ///
    /// Already-target tables are skipped. Every other table must match its exact
    /// predecessor or be authoritatively absent for a planned create. No writes
    /// occur until all requests are prepared. Each mutation is submitted once;
    /// after uncertainty, run reconciliation before restarting this operation.
    /// A restart always reloads all state, so completed tables are not repeated.
    /// The returned report is a fresh observation, not a cross-table lease.
    ///
    /// # Errors
    /// Rejects drift, unsupported conditional commits, invalid physical counters,
    /// catalog failures, uncertain mutations, and failed final readback.
    pub async fn apply(
        &self,
        catalog: &dyn RegistryDeploymentCatalog,
        principal: &Principal,
    ) -> Result<RegistryReconciliation, RegistryApplyError> {
        let mut mutations = Vec::new();
        for planned in &self.tables {
            match catalog.load_state(&planned.table, principal).await {
                Ok(state) if planned.target.matches_metadata(state.metadata()) => {}
                Ok(state) => match &planned.transition {
                    TableTransition::Update { previous }
                        if previous.matches_metadata(state.metadata()) =>
                    {
                        let token = state
                            .token()
                            .ok_or(RegistryApplyError::ConditionalCommitUnavailable)?
                            .to_owned();
                        let request = prepare_commit(planned, state.metadata())
                            .map_err(RegistryApplyError::Preflight)?;
                        let key = content_hash_json(&(
                            "querygraph.registry-deployment.v1",
                            &self.target_digest,
                            &planned.table,
                        ))
                        .map_err(RegistryApplyError::Preflight)?;
                        mutations.push(PreparedMutation::Update {
                            table: planned,
                            token,
                            request,
                            key,
                        });
                    }
                    _ => return Err(RegistryApplyError::Drift(planned.table.clone())),
                },
                Err(LakeCatError::NotFound {
                    object: "table", ..
                }) => match &planned.transition {
                    TableTransition::Create { request } => {
                        mutations.push(PreparedMutation::Create {
                            table: &planned.table,
                            request,
                        })
                    }
                    TableTransition::Update { .. } => {
                        return Err(RegistryApplyError::Drift(planned.table.clone()));
                    }
                },
                Err(error) => return Err(RegistryApplyError::Preflight(error)),
            }
        }
        for mutation in mutations {
            let (table, result) = match mutation {
                PreparedMutation::Create { table, request } => (
                    table,
                    catalog
                        .create_registry_table(table, principal, request)
                        .await,
                ),
                PreparedMutation::Update {
                    table,
                    token,
                    request,
                    key,
                } => (
                    &table.table,
                    catalog
                        .commit_registry_table(&table.table, principal, &token, &request, &key)
                        .await,
                ),
            };
            result.map_err(|source| RegistryApplyError::RequiresReconciliation {
                table: table.clone(),
                source,
            })?;
        }
        self.reconcile(catalog, principal)
            .await
            .map_err(RegistryApplyError::Verification)
    }
}

fn physical_counter(metadata: &Value, key: &str) -> Result<i64, LakeCatError> {
    metadata[key]
        .as_i64()
        .filter(|value| (0..=i64::from(i32::MAX)).contains(value))
        .ok_or_else(|| {
            LakeCatError::Conflict("catalog omitted a valid physical schema counter".into())
        })
}

fn prepare_commit(
    planned: &TableDeployment,
    metadata: &Value,
) -> Result<RegistryCatalogCommit, LakeCatError> {
    let schema_id = physical_counter(metadata, "current-schema-id")?;
    let last_field_id = physical_counter(metadata, "last-column-id")?;
    let uuid = metadata["table-uuid"]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| LakeCatError::Conflict("catalog omitted table UUID".into()))?;
    let requirements = vec![
        json!({"type":"assert-table-uuid","uuid":uuid}),
        json!({"type":"assert-current-schema-id","current-schema-id":schema_id}),
        json!({"type":"assert-last-assigned-field-id","last-assigned-field-id":last_field_id}),
    ];
    let mut updates = Vec::new();
    if !planned.target.matches_schema(metadata) {
        let schemas = metadata["schemas"]
            .as_array()
            .ok_or_else(|| LakeCatError::Conflict("catalog omitted schemas".into()))?;
        let max_schema_id = schemas
            .iter()
            .map(|schema| physical_counter(schema, "schema-id"))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .ok_or_else(|| LakeCatError::Conflict("catalog omitted schemas".into()))?;
        let next_schema_id = max_schema_id
            .checked_add(1)
            .filter(|id| *id <= i64::from(i32::MAX))
            .ok_or_else(|| LakeCatError::Conflict("catalog schema IDs exhausted".into()))?;
        let mut schema = planned.target.schema().clone();
        schema["schema-id"] = json!(next_schema_id);
        let fields = schema["fields"]
            .as_array()
            .ok_or_else(|| LakeCatError::Internal("invalid Pinax schema projection".into()))?;
        let previous_fields = schemas
            .iter()
            .find(|schema| schema["schema-id"] == schema_id)
            .and_then(|schema| schema["fields"].as_array())
            .ok_or_else(|| {
                LakeCatError::Conflict("catalog omitted current schema fields".into())
            })?;
        let mut next_last_id = last_field_id;
        for field in fields {
            let id = physical_counter(field, "id")?;
            let existing = previous_fields.iter().any(|previous| previous["id"] == id);
            if !existing && id <= last_field_id {
                return Err(LakeCatError::Conflict(
                    "new registry field reuses a previously assigned catalog ID".into(),
                ));
            }
            next_last_id = next_last_id.max(id);
        }
        updates.push(json!({"action":"add-schema","schema":schema,"last-column-id":next_last_id}));
        updates.push(json!({"action":"set-current-schema","schema-id":next_schema_id}));
    }
    updates.push(json!({"action":"set-properties","updates":planned.target.properties()}));
    Ok(RegistryCatalogCommit {
        requirements,
        updates,
    })
}

#[cfg(test)]
mod tests;
