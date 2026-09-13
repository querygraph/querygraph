//! Whole-registry deployment planning and authenticated reconciliation.
//!
//! Pinax owns compatibility, schema projection, and binding comparison. This
//! host layer enumerates every bound table, including unchanged tables whose
//! whole-registry digest changes. Plans are review artifacts, not write grants.
//! Reconciliation reads current catalog state and never retries or mutates it.

use lakecat_core::{LakeCatError, Principal, TableIdent};
use pinax::{
    PinaxError, Registry,
    adapters::{CatalogBinding, CatalogCreatePlan, CatalogTableContract},
};
use serde::Serialize;

use super::RegistryCatalog;

pub mod apply;
pub mod catalog;
pub mod cli;

/// Reviewable transition between two validated, compatible registry snapshots.
#[derive(Debug, Serialize)]
pub struct RegistryDeploymentPlan {
    version: &'static str,
    enterprise: String,
    previous_digest: String,
    target_digest: String,
    tables: Vec<TableDeployment>,
}

#[derive(Debug, Serialize)]
struct TableDeployment {
    table: TableIdent,
    target: CatalogTableContract,
    transition: TableTransition,
}

#[derive(Debug, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum TableTransition {
    Create { request: CatalogCreatePlan },
    Update { previous: CatalogTableContract },
}

/// Observed binding of one table to the reviewed registry transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TableDeploymentStatus {
    /// Schema and all registry pins already match the target.
    Target,
    /// A new table has not yet been created.
    PendingCreate,
    /// The existing table still matches the predecessor's schema and pins.
    PendingUpdate,
    /// An existing predecessor table unexpectedly disappeared.
    MissingExisting,
    /// The current schema or pins match neither reviewed endpoint.
    Drift,
}

/// Aggregate state observed during a reconciliation pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryDeploymentStatus {
    /// Every table was observed at the target schema and pins.
    Ready,
    /// At least one table remains at its known predecessor or awaits creation.
    Pending,
    /// Drift or an unexpectedly missing predecessor requires investigation.
    Blocked,
}

#[derive(Debug, Serialize)]
struct TableReconciliation {
    table: TableIdent,
    status: TableDeploymentStatus,
}

/// Read-only observations for all tables in the transition, without physical
/// storage locations or credentials. This is not an atomic catalog snapshot or
/// permission to switch consumers while other writers are active.
#[derive(Debug, Serialize)]
pub struct RegistryReconciliation {
    version: &'static str,
    previous_digest: String,
    target_digest: String,
    status: RegistryDeploymentStatus,
    tables: Vec<TableReconciliation>,
}

impl RegistryReconciliation {
    /// Whether this full pass observed ready, pending, or conflicting state.
    pub fn status(&self) -> RegistryDeploymentStatus {
        self.status
    }
}

/// A catalog read failed; partial observations must not be presented as ready.
#[derive(Debug, thiserror::Error)]
pub enum RegistryReconciliationError {
    /// The configured catalog denied access or failed to return authoritative state.
    #[error("registry reconciliation catalog read failed")]
    Catalog(#[source] LakeCatError),
}

impl RegistryDeploymentPlan {
    /// Plan a compatible transition, including the pin updates for unchanged tables.
    ///
    /// # Errors
    /// Rejects incompatible revisions, unknown identities, invalid new-table
    /// create layouts, and serialization failures. Existing sparse IDs are
    /// preserved through Pinax's table-contract API.
    pub fn new(
        previous: &Registry,
        target: &Registry,
        binding: &CatalogBinding,
    ) -> Result<Self, PinaxError> {
        previous.check_successor(target)?;
        let tables = target
            .tables()
            .iter()
            .map(|table| {
                let transition = match previous.table(&table.name) {
                    Ok(_) => TableTransition::Update {
                        previous: binding.table_contract(previous, &table.name)?,
                    },
                    Err(PinaxError::NotFound(_)) => TableTransition::Create {
                        request: binding.create_plan(target, &table.name)?,
                    },
                    Err(error) => return Err(error),
                };
                Ok(TableDeployment {
                    table: binding.table_ident(target, &table.name)?,
                    target: binding.table_contract(target, &table.name)?,
                    transition,
                })
            })
            .collect::<Result<Vec<_>, PinaxError>>()?;
        Ok(Self {
            version: "querygraph.registry-deployment.v1",
            enterprise: target.enterprise().into(),
            previous_digest: previous.digest()?,
            target_digest: target.digest()?,
            tables,
        })
    }

    /// Read every bound table and classify target, predecessor, absence, or drift.
    ///
    /// Run this after an uncertain catalog write before deciding what to do next.
    /// The pass makes no writes and never retries. It remains cancellable at each
    /// catalog await. Hold an owner-enforced maintenance window when using the
    /// report as a cutover gate; observations alone do not exclude other writers.
    ///
    /// # Errors
    /// Any catalog error other than authoritative table-not-found aborts the pass,
    /// preserving denial/unavailability as errors instead of apparent readiness.
    pub async fn reconcile(
        &self,
        catalog: &dyn RegistryCatalog,
        principal: &Principal,
    ) -> Result<RegistryReconciliation, RegistryReconciliationError> {
        let mut tables = Vec::with_capacity(self.tables.len());
        let mut aggregate = RegistryDeploymentStatus::Ready;
        for planned in &self.tables {
            let status = match catalog.load_metadata(&planned.table, principal).await {
                Ok(metadata) if planned.target.matches_metadata(&metadata) => {
                    TableDeploymentStatus::Target
                }
                Ok(metadata) => match &planned.transition {
                    TableTransition::Update { previous }
                        if previous.matches_metadata(&metadata) =>
                    {
                        TableDeploymentStatus::PendingUpdate
                    }
                    TableTransition::Update { .. } | TableTransition::Create { .. } => {
                        TableDeploymentStatus::Drift
                    }
                },
                Err(LakeCatError::NotFound {
                    object: "table", ..
                }) => match planned.transition {
                    TableTransition::Create { .. } => TableDeploymentStatus::PendingCreate,
                    TableTransition::Update { .. } => TableDeploymentStatus::MissingExisting,
                },
                Err(error) => return Err(RegistryReconciliationError::Catalog(error)),
            };
            aggregate = match (aggregate, status) {
                (_, TableDeploymentStatus::MissingExisting | TableDeploymentStatus::Drift) => {
                    RegistryDeploymentStatus::Blocked
                }
                (RegistryDeploymentStatus::Blocked, _) => RegistryDeploymentStatus::Blocked,
                (
                    _,
                    TableDeploymentStatus::PendingCreate | TableDeploymentStatus::PendingUpdate,
                ) => RegistryDeploymentStatus::Pending,
                (previous, TableDeploymentStatus::Target) => previous,
            };
            tables.push(TableReconciliation {
                table: planned.table.clone(),
                status,
            });
        }
        Ok(RegistryReconciliation {
            version: "querygraph.registry-reconciliation.v1",
            previous_digest: self.previous_digest.clone(),
            target_digest: self.target_digest.clone(),
            status: aggregate,
            tables,
        })
    }
}

#[cfg(test)]
mod tests;
