//! cynic-vcs — Sovereign Version Control System domain and runners for CYNIC.
//!
//! One crate per cohesive domain (R7): pure `types`, `policy`, `sync`,
//! `worktree`, `release`, `bootstrap`, `materialization`, `inventory` and
//! `publication` modules, with every Forgejo transport concern isolated in
//! `adapters/forgejo` and port traits in `ports`. Domain modules are zero-I/O.

// Canonical types owned by lower layers.
pub use cynic_protocol::vcs::{
    SOURCE_MATERIALIZATION_FORMAT, SourceSnapshot, source_worktree_lease_target,
};

pub mod adapters;
pub mod bootstrap;
pub mod inventory;
pub mod materialization;
pub mod policy;
pub mod ports;
pub mod publication;
pub mod release;
pub mod sync;
pub mod types;
pub mod worktree;

// Internal sub-domains retained as direct modules.
pub mod development_genesis;
pub mod git_hygiene;
pub mod git_topology;
pub mod isolated_worktree;
pub mod pull_request;

pub use bootstrap::*;
pub use inventory::*;
pub use materialization::*;
pub use policy::*;
pub use ports::*;
pub use publication::*;
pub use release::*;
pub use sync::*;
pub use types::*;
pub use worktree::*;

pub use development_genesis::*;
pub use git_hygiene::*;
pub use git_topology::*;
pub use isolated_worktree::*;
pub use pull_request::*;

// Compatibility projections: the former public module paths, now thin `pub use`
// over the consolidated modules. No logic lives here.
pub mod source_control {
    pub use crate::policy::{admit_source_change, admit_source_stage};
    pub use crate::types::*;
}

pub mod forgejo_token {
    pub use crate::adapters::forgejo::{
        plan_forgejo_token_convergence, validate_forgejo_token_spec,
    };
    pub use crate::types::*;
}

pub mod forgejo {
    pub use crate::adapters::forgejo::plan_forgejo_convergence;
    pub use crate::types::*;
}

pub mod source_worktree {
    pub use crate::types::{
        ExecutedActionReceipt, ObservedWorktreeSnapshot, WorktreeConvergenceAction,
        WorktreeConvergenceApplyProof, WorktreeConvergencePlan, WorktreeConvergenceSummary,
    };
    pub use crate::worktree::reconcile_worktree_convergence_pure;
}

pub mod reconcile {
    pub use crate::types::*;
    pub use crate::worktree::{
        SOURCE_WORKTREE_RECONCILE_SCHEMA, VcsLockPort, apply_worktree_convergence,
        reconcile_source_worktrees,
    };
}
