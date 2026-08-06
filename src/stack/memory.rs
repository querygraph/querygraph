//! Marciana memory implementation behind the QueryGraph stack boundary.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use querygraph_memory::TursoMemoryStore;
use serde_json::Value;
use typesec_agent::interop::{ToolCallGuard, ToolCallRequest};
use typesec_core::policy::{PolicyEngine, RequestContext, SubjectId};
use typesec_memory::agent::{MemoryToolRouter, memory_bindings};
use typesec_memory::{MemoryError, MemoryVault};

/// Persistent memory service shared by QueryGraph adapters.
pub struct MemoryApi {
    guard: ToolCallGuard,
    router: MemoryToolRouter<TursoMemoryStore>,
}

/// A memory request was denied or failed after authentication.
#[derive(Debug, thiserror::Error)]
pub enum MemoryApiError {
    #[error("{0}")]
    Denied(String),
    #[error("{0}")]
    Failed(String),
}

impl MemoryApi {
    /// Open a file-backed Marciana/Grust store with a TypeSec RBAC policy.
    pub fn open(database: impl AsRef<Path>, policy_yaml: &str) -> Result<Self> {
        let engine: Arc<dyn PolicyEngine> = Arc::new(
            typesec_rbac::RbacEngine::from_yaml(policy_yaml)
                .map_err(|error| anyhow::anyhow!("parsing memory RBAC policy: {error}"))?,
        );
        let store = TursoMemoryStore::open(database.as_ref())
            .map_err(|error| anyhow::anyhow!("opening memory database: {error}"))?;
        let vault = MemoryVault::new(store).with_policy(engine.clone());
        let router = MemoryToolRouter::new(vault, engine.clone());
        let guard = memory_bindings()
            .into_iter()
            .fold(ToolCallGuard::new(engine), ToolCallGuard::bind);
        Ok(Self { guard, router })
    }

    /// Execute one normalized memory verb for an already verified subject.
    pub fn execute(
        &self,
        subject: &str,
        tool_name: &str,
        arguments: Value,
        purpose: Option<&str>,
    ) -> Result<Value, MemoryApiError> {
        let context = purpose.map_or_else(RequestContext::default, |purpose| {
            RequestContext::default().with_purpose(purpose)
        });
        let guarded = self.guard.check(
            &SubjectId::from(subject),
            ToolCallRequest::new(tool_name, arguments),
            &context,
        );
        if !guarded.verdict.is_allowed() {
            return Err(MemoryApiError::Denied(
                guarded
                    .denial_message()
                    .unwrap_or_else(|| "memory request was not authorized".to_string()),
            ));
        }
        self.router
            .handle(subject, &guarded.request, &context)
            .map_err(memory_api_error)
    }
}

pub(crate) fn memory_api_error(error: MemoryError) -> MemoryApiError {
    match error {
        MemoryError::PolicyDenied { .. }
        | MemoryError::Capability(_)
        | MemoryError::SpaceMismatch { .. }
        | MemoryError::AboveCeiling { .. }
        | MemoryError::GovernedSourceScopeMismatch => MemoryApiError::Denied(error.to_string()),
        MemoryError::NotFound(_)
        | MemoryError::GovernedSourceVerification(_)
        | MemoryError::Store(_)
        | MemoryError::Cognition(_)
        | MemoryError::CognitionCommit(_)
        | MemoryError::CognitionRecovery(_) => MemoryApiError::Failed(error.to_string()),
    }
}
