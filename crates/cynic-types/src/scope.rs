//! Resource scope for claims and proofs.

use serde::{Deserialize, Serialize};

/// A scoped resource — file, module, or crate boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub kind: ScopeKind,
    pub path: String,
}

/// Granularity of a scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScopeKind {
    /// A single file path.
    File,
    /// A Rust module path (e.g. `crate::module::sub`).
    Module,
    /// A crate-level boundary.
    Crate,
}
