//! Compatibility exports for QueryGraph's persistent memory API.
//!
//! Implementation ownership lives in `stack::memory`; this module preserves
//! the existing public path for callers and route compatibility.

#[cfg(test)]
pub(crate) use crate::stack::memory::memory_api_error;
pub use crate::stack::memory::{MemoryApi, MemoryApiError};
