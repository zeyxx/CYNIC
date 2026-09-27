//! cynic-vcs — Sovereign Version Control System domain and runners for CYNIC.

pub use cynic_protocol::vcs::{
    SOURCE_MATERIALIZATION_FORMAT, SourceSnapshot, source_worktree_lease_target,
};

// Internal modules
pub mod development_genesis;
pub mod entry;
pub mod forgejo;
pub mod forgejo_port;
pub mod forgejo_token;
pub mod git_hygiene;
pub mod git_topology;
pub mod isolated_worktree;
pub mod parse;
pub mod pull_request;
pub mod reconcile;
pub mod runner;
pub mod source_control;
pub mod source_materialization;
pub mod source_policy;
pub mod source_release;
pub mod source_repository_bootstrap;
pub mod source_worktree;
pub mod source_worktree_inventory;
pub mod sync;

#[cfg(test)]
mod source_worktree_inventory_tests;

use std::path::{Path, PathBuf};

pub use development_genesis::*;
pub use forgejo::*;
pub use forgejo_port::{ForgejoObservationPort, build_forgejo_plan, forgejo_token_spec};
pub use git_hygiene::*;
pub use git_topology::*;
pub use parse::*;
pub use runner::*;
pub use source_control::*;
pub use source_materialization::*;
pub use source_policy::*;
pub use source_release::*;
pub use source_repository_bootstrap::*;
pub use source_worktree::*;
pub use source_worktree_inventory::*;

const MAX_PORCELAIN_BYTES: usize = 4 * 1024 * 1024;

/// Observe worktrees of a Git repository via a `VcsRunner`.
pub fn observe_source_worktree_inventory(
    repository_root: &Path,
    workspace_store: &Path,
    repository_id: &str,
    node_id: &str,
    runner: &impl VcsRunner,
    observed_at_ms: u64,
) -> SourceWorktreeInventoryObservation {
    let root = repository_root.to_string_lossy().to_string();
    let argv = ["git", "-C", &root, "worktree", "list", "--porcelain", "-z"];
    let output = runner.run(&argv);

    if output.exit_code != Some(0) || output.spawn_error.is_some() {
        return unknown_observation(
            repository_id,
            node_id,
            observed_at_ms,
            SourceWorktreeInventoryObservationUnknown::GitCommandFailed,
        );
    }

    if output.stdout.len() > MAX_PORCELAIN_BYTES {
        return unknown_observation(
            repository_id,
            node_id,
            observed_at_ms,
            SourceWorktreeInventoryObservationUnknown::OutputLimitExceeded,
        );
    }

    let text = match String::from_utf8(output.stdout) {
        Ok(text) => text,
        Err(error) => {
            return malformed_observation(
                repository_id,
                node_id,
                observed_at_ms,
                cynic_protocol::digest::digest_bytes(error.as_bytes()),
            );
        }
    };

    let repository_root = observed_absolute_path(repository_root);
    let workspace_store = observed_absolute_path(workspace_store);
    let (entries, entry_limit_exceeded, malformed_record_evidence) =
        parse_porcelain(&text, &repository_root, &workspace_store);
    let mut evidence_refs: Vec<String> =
        entries.iter().map(|e| e.evidence_digest.clone()).collect();
    evidence_refs.extend(malformed_record_evidence.iter().cloned());
    let mut observation_unknowns = Vec::new();
    if entry_limit_exceeded {
        observation_unknowns.push(SourceWorktreeInventoryObservationUnknown::EntryLimitExceeded);
    }
    if !malformed_record_evidence.is_empty() {
        observation_unknowns.push(SourceWorktreeInventoryObservationUnknown::MalformedRecord);
    }
    if entries.is_empty() && observation_unknowns.is_empty() {
        observation_unknowns.push(SourceWorktreeInventoryObservationUnknown::EmptyOutput);
    }

    SourceWorktreeInventoryObservation {
        schema: SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
        repository_id: repository_id.to_string(),
        node_id: node_id.to_string(),
        observed_at_ms,
        entries,
        evidence_refs,
        observation_unknowns,
    }
}

fn unknown_observation(
    repository_id: &str,
    node_id: &str,
    observed_at_ms: u64,
    unknown: SourceWorktreeInventoryObservationUnknown,
) -> SourceWorktreeInventoryObservation {
    SourceWorktreeInventoryObservation {
        schema: SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
        repository_id: repository_id.to_string(),
        node_id: node_id.to_string(),
        observed_at_ms,
        entries: Vec::new(),
        evidence_refs: Vec::new(),
        observation_unknowns: vec![unknown],
    }
}

fn malformed_observation(
    repository_id: &str,
    node_id: &str,
    observed_at_ms: u64,
    evidence_digest: String,
) -> SourceWorktreeInventoryObservation {
    SourceWorktreeInventoryObservation {
        schema: SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
        repository_id: repository_id.to_string(),
        node_id: node_id.to_string(),
        observed_at_ms,
        entries: Vec::new(),
        evidence_refs: vec![evidence_digest],
        observation_unknowns: vec![SourceWorktreeInventoryObservationUnknown::MalformedRecord],
    }
}

fn observed_absolute_path(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}
