//! Trusted catalog boundary for conditional registry deployment.

use async_trait::async_trait;
use lakecat_core::{LakeCatError, LakeCatResult, Principal, TableIdent};
use pinax::adapters::CatalogCreatePlan;
use serde::Serialize;
use serde_json::Value;

use crate::registry_service::RegistryCatalog;

/// Metadata and an optional opaque state token obtained by a trusted adapter.
/// This observation is not an authorization capability or a maintenance lease.
pub struct CatalogTableState {
    metadata: Value,
    token: Option<String>,
}

impl CatalogTableState {
    /// Construct an observation inside a trusted catalog adapter.
    ///
    /// # Errors
    /// Rejects non-object metadata and malformed LakeCat state tokens.
    pub fn new(metadata: Value, token: Option<String>) -> LakeCatResult<Self> {
        if !metadata.is_object() {
            return Err(LakeCatError::Internal(
                "catalog omitted table metadata".into(),
            ));
        }
        if token.as_deref().is_some_and(|token| {
            !token.strip_prefix("sha256:").is_some_and(|digest| {
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        }) {
            return Err(LakeCatError::Internal("invalid catalog state token".into()));
        }
        Ok(Self { metadata, token })
    }

    /// Metadata returned by the same catalog read as the token.
    pub fn metadata(&self) -> &Value {
        &self.metadata
    }

    /// Copy this opaque value to the catalog's conditional commit boundary.
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}

/// Standard Iceberg updates derived from a reviewed Pinax target contract.
/// Only the deployment planner constructs this request; Sail validates/applies it.
#[derive(Debug, Serialize)]
pub struct RegistryCatalogCommit {
    pub(super) requirements: Vec<Value>,
    pub(super) updates: Vec<Value>,
}

/// Host-selected mutation adapter. Every mutation must authenticate `principal`.
/// Conditional commits must enforce the token at the catalog owner, atomically
/// with its existing publication CAS; clients cannot emulate that guarantee.
#[async_trait]
pub trait RegistryDeploymentCatalog: RegistryCatalog {
    /// Fresh read; only an authoritative missing-table error establishes absence.
    async fn load_state(
        &self,
        table: &TableIdent,
        principal: &Principal,
    ) -> LakeCatResult<CatalogTableState>;

    /// Create a new table without replacing an existing catalog entry.
    async fn create_registry_table(
        &self,
        table: &TableIdent,
        principal: &Principal,
        request: &CatalogCreatePlan,
    ) -> LakeCatResult<()>;

    /// Submit one conditional commit without retrying an uncertain outcome.
    async fn commit_registry_table(
        &self,
        table: &TableIdent,
        principal: &Principal,
        token: &str,
        request: &RegistryCatalogCommit,
        idempotency_key: &str,
    ) -> LakeCatResult<()>;
}
