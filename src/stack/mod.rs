//! QueryGraph-owned boundaries around TypeSec and Marciana.
//!
//! Application layers depend on these small interfaces instead of importing
//! upstream implementation types throughout the service.

pub mod cognition;
pub mod error;
pub mod memory;
pub mod security;

pub use self::error::StackError;
pub use self::security::{AuthFailure, VerifiedAgent, verify_http_envelope};
