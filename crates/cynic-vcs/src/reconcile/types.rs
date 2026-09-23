//! Types and decisions for source worktree reconciliation and multi-agent Git convergence.
//!
//! Re-exports canonical models from `crate::source_worktree`.

pub use crate::source_worktree::*;

pub const SOURCE_WORKTREE_RECONCILE_SCHEMA: &str = SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA;
