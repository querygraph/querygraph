//! Stable errors exposed by the QueryGraph stack facade.

/// Product-level error categories for TypeSec and Marciana operations.
#[derive(Debug, thiserror::Error)]
pub enum StackError {
    /// The request could not be authenticated or verified.
    #[error("authentication failed: {0}")]
    Authentication(String),
    /// The verified identity lacks the required authority.
    #[error("authorization denied: {0}")]
    Authorization(String),
    /// The request is malformed or outside the supported contract.
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// A durable memory operation failed.
    #[error("memory operation failed: {0}")]
    Memory(String),
    /// A cognition operation failed.
    #[error("cognition operation failed: {0}")]
    Cognition(String),
    /// A receipt or recovery operation failed.
    #[error("receipt or recovery failed: {0}")]
    Recovery(String),
}
