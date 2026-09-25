//! Source worktree inventory — re-exported from `cynic-vcs` per Foundation §481 (SoC).

pub use cynic_vcs::{
    VcsCommandOutput as SourceWorktreeInventoryCommandOutput,
    VcsRunner as SourceWorktreeInventoryRunner, observe_source_worktree_inventory, parse_porcelain,
};
pub use crate::adapters::vcs_runner::LocalVcsRunner as LocalSourceWorktreeInventoryRunner;
