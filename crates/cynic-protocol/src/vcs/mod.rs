//! Version control, source trees, Forgejo and git hygiene contracts.
//!
//! Sub-domain grouping:
//! - `forgejo`: Forgejo instance metadata, API client models, and webhook signatures.
//! - `forgejo_token`: Forgejo access token authorization and rotation contracts.
//! - `git_hygiene`: branch invariants, dirty working tree, and untracked file predicates.
//! - `git_topology`: Git commit graph invariants, branch protection, and lineage rules.
//! - `source_control`: repository state, change set declarations, and sync boundaries.
//! - `source_materialization`: file materialization and bundle expansion policies.
//! - `source_policy`: branch policies, protected branches, and commit signoff rules.
//! - `source_release`: release tag policies, sovereign publication, and readback.
//! - `source_repository_bootstrap`: initial repo layout bootstrap and remote syncing.
//! - `source_worktree`: agent workspace leases, pruning, and dirty worktree protection.
//! - `source_worktree_inventory`: worktree audit records, candidate validation, and unknowns.

pub mod forgejo;
pub mod forgejo_token;
pub mod git_hygiene;
pub mod git_topology;
pub mod source_control;
pub mod source_materialization;
pub mod source_policy;
pub mod source_release;
pub mod source_repository_bootstrap;
pub mod source_worktree;
pub mod source_worktree_inventory;

#[cfg(test)]
mod source_worktree_inventory_tests;

pub use forgejo::*;
pub use forgejo_token::*;
pub use git_hygiene::*;
pub use git_topology::*;
pub use source_control::*;
pub use source_materialization::*;
pub use source_policy::*;
pub use source_release::*;
pub use source_repository_bootstrap::*;
pub use source_worktree::*;
pub use source_worktree_inventory::*;
