//! Source worktree inventory — re-exported from `cynic-vcs` per Foundation §481 (SoC).

pub use cynic_vcs::{
    LocalVcsRunner as LocalSourceWorktreeInventoryRunner,
    VcsCommandOutput as SourceWorktreeInventoryCommandOutput,
    VcsRunner as SourceWorktreeInventoryRunner, observe_source_worktree_inventory, parse_porcelain,
};
