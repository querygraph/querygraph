//! Trusted Pinax service composition for transport adapters.
//!
//! Registry validity and scan authorization belong to Pinax. This module owns
//! request authentication, deployment configuration, and catalog acquisition.
//! No agent argument can select a policy engine or supply catalog metadata.
//! All I/O is awaited directly; cancellation drops the backend operation.

use std::sync::Arc;

use async_trait::async_trait;
use lakecat_core::sail::{SailCatalogEngine, ScanPlan};
use lakecat_core::{LakeCatError, LakeCatResult, Principal, PrincipalKind, TableIdent};
use pinax::adapters::{
    CatalogBinding, DiscoveryCursor, DiscoveryPage, GovernedRegistry, PinaxScanError, ScanIntent,
};
use pinax::{PinaxError, Registry};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use typesec_core::policy::PolicyEngine;

use crate::agent::PyTypeDidEnvelope;
use crate::stack::security::verify_http_envelope;

pub mod config;
pub mod deployment;
mod execution;
pub mod http;
pub mod ontology;
pub use execution::RegistryExecution;

/// Maximum signed scan-intent JSON size, independent of the registry size.
pub const MAX_SCAN_INTENT_BYTES: usize = 64 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiscoveryIntent {
    purpose: String,
    cursor: Option<DiscoveryCursor>,
    limit: usize,
}

enum RegistryOperation {
    Ontology,
    Discover,
    Plan,
    Execute,
}

impl RegistryOperation {
    fn resource(&self) -> &'static str {
        match self {
            Self::Discover => "/mcp/discover_pinax_tables",
            Self::Ontology => "/mcp/discover_pinax_ontology",
            Self::Plan => "/mcp/plan_pinax_scan",
            Self::Execute => "/mcp/execute_pinax_scan",
        }
    }
}

/// Deployment-owned catalog reader and planner. Implementations must authenticate
/// the supplied principal at their remote boundary and fetch current metadata.
#[async_trait]
pub trait RegistryCatalog: Send + Sync {
    /// Observe the catalog metadata and concurrency token used for execution.
    /// Unsupported adapters fail closed instead of executing an unbound plan.
    async fn load_execution_state(
        &self,
        _table: &TableIdent,
        _principal: &Principal,
    ) -> LakeCatResult<deployment::catalog::CatalogTableState> {
        Err(LakeCatError::NotSupported(
            "governed execution is unavailable".into(),
        ))
    }

    /// Ask the authenticated owner to execute the observed, purpose-bound scope.
    /// The owner must enforce current execution authority and release checks.
    async fn execute_governed(
        &self,
        _table: &TableIdent,
        _principal: &Principal,
        _state: &deployment::catalog::CatalogTableState,
        _intent: &ScanIntent,
        _snapshot: i64,
    ) -> LakeCatResult<Value> {
        Err(LakeCatError::NotSupported(
            "governed execution is unavailable".into(),
        ))
    }
    /// Read current catalog metadata without trusting request-provided metadata.
    async fn load_metadata(
        &self,
        table: &TableIdent,
        principal: &Principal,
    ) -> LakeCatResult<Value>;

    /// The real Sail planner, or an explicit unavailable implementation.
    fn engine(&self) -> &dyn SailCatalogEngine;
}

/// Deployment-owned TypeSec engine and its exact contract identity.
pub struct RegistryPolicy {
    id: String,
    revision: u32,
    engine: Arc<dyn PolicyEngine>,
}

impl RegistryPolicy {
    /// Bind an engine to the policy identity configured by its owner.
    ///
    /// # Errors
    /// Rejects empty identities and zero revisions.
    pub fn new(
        id: String,
        revision: u32,
        engine: Arc<dyn PolicyEngine>,
    ) -> Result<Self, RegistryServiceError> {
        if id.trim().is_empty() || revision == 0 {
            return Err(RegistryServiceError::Configuration);
        }
        Ok(Self {
            id,
            revision,
            engine,
        })
    }
}

/// Immutable registry, policy, and catalog composition shared by requests.
pub struct RegistryService {
    ontology: Option<ontology::CentralOntology>,
    registry: Registry,
    binding: CatalogBinding,
    policy: RegistryPolicy,
    catalog: Arc<dyn RegistryCatalog>,
    server_did: String,
}

/// A plan authenticated and authorized by the host. This is not an execution
/// capability and cannot be constructed by deserializing agent JSON.
pub struct PreparedRegistryScan {
    summary: RegistryScanSummary,
    plan: ScanPlan,
    principal: Principal,
}

/// Safe agent-visible planning evidence; excludes storage paths and scan tasks.
#[derive(Debug, Serialize)]
pub struct RegistryScanSummary {
    enterprise: String,
    table: String,
    revision: u32,
    registry_digest: String,
    snapshot_id: i64,
    columns: Vec<String>,
    purpose: String,
    limit: u64,
    planned_by: String,
}

impl PreparedRegistryScan {
    /// Agent-visible evidence for the authorized projection.
    pub fn summary(&self) -> &RegistryScanSummary {
        &self.summary
    }

    /// Borrow tasks for a trusted host's execution integration. Possession of
    /// this plan does not replace LakeCat execution authorization/revalidation.
    pub fn plan(&self) -> &ScanPlan {
        &self.plan
    }

    /// Identity verified from the signed request, never an argument override.
    pub fn principal(&self) -> &Principal {
        &self.principal
    }
}

/// Service failures retain authentication, contract, policy, drift, and outage
/// distinctions. Transports must redact backend error sources.
#[derive(Debug, thiserror::Error)]
pub enum RegistryServiceError {
    /// Central ontology is invalid, unavailable, or differs from its reviewed pin.
    #[error("central ontology unavailable or stale")]
    Ontology(#[from] pinax::ontology::OntologyError),
    /// Invalid deployment configuration.
    #[error("invalid registry service configuration")]
    Configuration,
    /// The envelope did not bind the caller, recipient, operation, and body.
    #[error("registry request authentication failed")]
    Authentication,
    /// Untrusted intent exceeded the input bound.
    #[error("registry scan intent exceeds size limit")]
    IntentLimit,
    /// The signed intent was not a valid scan request.
    #[error("invalid registry scan intent JSON")]
    Json(#[from] serde_json::Error),
    /// A logical registry identity was invalid or absent.
    #[error("invalid registry contract")]
    Contract(#[from] PinaxError),
    /// The trusted catalog failed to load metadata.
    #[error("registry catalog unavailable")]
    Catalog(#[source] LakeCatError),
    /// Pinax rejected authorization, schema/snapshot pins, or planning.
    #[error("registry scan rejected")]
    Scan(#[from] PinaxScanError),
}

impl RegistryService {
    fn governed(&self) -> GovernedRegistry<'_> {
        GovernedRegistry::new(
            &self.registry,
            &self.binding,
            self.policy.engine.as_ref(),
            &self.policy.id,
            self.policy.revision,
        )
    }

    fn authenticate(
        &self,
        intent_json: &str,
        envelope: &PyTypeDidEnvelope,
        operation: RegistryOperation,
    ) -> Result<Principal, RegistryServiceError> {
        if intent_json.len() > MAX_SCAN_INTENT_BYTES {
            return Err(RegistryServiceError::IntentLimit);
        }
        let verified = verify_http_envelope(
            operation.resource(),
            intent_json.as_bytes(),
            envelope,
            &self.server_did,
        )
        .map_err(|_| RegistryServiceError::Authentication)?;
        Principal::new(verified.subject, PrincipalKind::Agent)
            .map_err(|_| RegistryServiceError::Authentication)
    }

    /// Authenticate a discovery intent and return only Pinax-authorized metadata.
    ///
    /// Sign exact JSON bytes containing `purpose`, `limit` (1–100), and optional
    /// `cursor`, binding `/mcp/discover_pinax_tables`. No catalog/Sail I/O occurs;
    /// the response describes authorized registry metadata, not deployment state.
    ///
    /// # Errors
    /// Rejects invalid authentication, malformed or oversized intents, stale
    /// cursors, policy binding mismatches and unresolved decisions.
    pub async fn discover(
        &self,
        intent_json: &str,
        envelope: &PyTypeDidEnvelope,
    ) -> Result<DiscoveryPage, RegistryServiceError> {
        let principal = self.authenticate(intent_json, envelope, RegistryOperation::Discover)?;
        let intent: DiscoveryIntent = serde_json::from_str(intent_json)?;
        Ok(self
            .governed()
            .discover(
                &principal,
                &intent.purpose,
                intent.cursor.as_ref(),
                intent.limit,
            )
            .await?)
    }

    /// Compose only deployment-owned configuration with a validated snapshot.
    ///
    /// # Errors
    /// Rejects an empty server identity. Policy binding is checked by Pinax
    /// on each request so a mismatched table fails closed.
    pub fn new(
        registry: Registry,
        binding: CatalogBinding,
        policy: RegistryPolicy,
        catalog: Arc<dyn RegistryCatalog>,
        server_did: String,
    ) -> Result<Self, RegistryServiceError> {
        if server_did.trim().is_empty() {
            return Err(RegistryServiceError::Configuration);
        }
        Ok(Self {
            ontology: None,
            registry,
            binding,
            policy,
            catalog,
            server_did,
        })
    }

    /// Authenticate exact intent bytes, fetch trusted metadata, and delegate all
    /// scan policy and schema decisions to the released Pinax adapter.
    ///
    /// Sign `invoke` on `/mcp/plan_pinax_scan`, addressed to the configured
    /// server DID, with `payload.bodySha256` equal to SHA-256 of `intent_json`.
    /// The envelope may be replayed only as a new read request: policy, metadata,
    /// and snapshot checks run again each time; no grant is cached or minted.
    ///
    /// # Errors
    /// Rejects invalid authentication before catalog I/O, malformed intents,
    /// catalog failures, policy denials/unresolved decisions, drift, and backend
    /// failures. No rows are read and no write or retry is performed.
    pub async fn plan_scan(
        &self,
        intent_json: &str,
        envelope: &PyTypeDidEnvelope,
    ) -> Result<PreparedRegistryScan, RegistryServiceError> {
        let principal = self.authenticate(intent_json, envelope, RegistryOperation::Plan)?;
        let intent: ScanIntent = serde_json::from_str(intent_json)?;
        let governed = self.governed();
        governed.authorize_scan(&principal, &intent).await?;
        let table = self.binding.table_ident(&self.registry, &intent.table)?;
        let metadata = self
            .catalog
            .load_metadata(&table, &principal)
            .await
            .map_err(RegistryServiceError::Catalog)?;
        let plan = governed
            .plan_scan(self.catalog.engine(), &principal, &metadata, &intent)
            .await?;
        // Planning may race a schema or registry publication without changing
        // the data snapshot. Observe catalog state again before reporting a plan.
        let current = self
            .catalog
            .load_metadata(&table, &principal)
            .await
            .map_err(RegistryServiceError::Catalog)?;
        if current != metadata {
            return Err(PinaxScanError::Drift.into());
        }
        let snapshot_id = plan.snapshot_id.ok_or(PinaxScanError::Drift)?;
        let summary = RegistryScanSummary {
            enterprise: self.registry.enterprise().to_owned(),
            revision: self.registry.table(&intent.table)?.revision,
            registry_digest: self.registry.digest()?,
            table: intent.table,
            columns: intent.columns,
            purpose: intent.purpose,
            limit: intent.limit,
            snapshot_id,
            planned_by: plan.planned_by.clone(),
        };
        Ok(PreparedRegistryScan {
            summary,
            plan,
            principal,
        })
    }
}

#[cfg(test)]
mod tests;
