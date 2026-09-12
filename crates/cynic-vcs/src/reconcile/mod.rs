//! Source worktree reconciliation and multi-agent Git convergence.
//!
//! Exposes pure plan reconciliation (`reconcile_source_worktrees`) and bounded action
//! execution (`apply_worktree_convergence`) with full read-back proof.

pub mod apply;
pub mod reconcile;
pub mod types;

pub use apply::apply_worktree_convergence;
pub use reconcile::reconcile_source_worktrees;
pub use types::*;
