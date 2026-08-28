use std::path::Path;

use cynic_protocol::digest::digest_json;
use cynic_protocol::source_worktree_inventory::{
    SourceWorktreeInventoryEntry, SourceWorktreeInventoryEntryState,
    SourceWorktreeInventoryObservation, SourceWorktreeInventoryPathBasis,
    SourceWorktreeInventoryUnknown, validate_source_worktree_inventory_observation,
    SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA,
};

use crate::update_lifecycle::ArgvRunner;

pub fn observe_source_worktree_inventory(
    repository_root: &Path,
    repository_id: &str,
    node_id: &str,
    runner: &impl ArgvRunner,
) -> SourceWorktreeInventoryObservation {
    let root = repository_root.to_string_lossy().to_string();
    let argv = ["git", "-C", &root, "worktree", "list", "--porcelain"];
    let output = runner.run(&argv);
    let observed_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    if output.exit_code != Some(0) || output.spawn_error.is_some() {
        return SourceWorktreeInventoryObservation {
            schema: SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
            repository_id: repository_id.to_string(),
            node_id: node_id.to_string(),
            observed_at_ms,
            entries: Vec::new(),
            evidence_refs: Vec::new(),
        };
    }

    let text = if output.stdout.is_empty() {
        output.stderr
    } else {
        output.stdout
    };

    let entries = parse_porcelain(&text, repository_root);
    let evidence_refs: Vec<String> = entries.iter().map(|e| e.evidence_digest.clone()).collect();

    SourceWorktreeInventoryObservation {
        schema: SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
        repository_id: repository_id.to_string(),
        node_id: node_id.to_string(),
        observed_at_ms,
        entries,
        evidence_refs,
    }
}

fn parse_porcelain(text: &str, repository_root: &Path) -> Vec<SourceWorktreeInventoryEntry> {
    let mut entries = Vec::new();
    let _root_canonical = repository_root.display().to_string();
    let mut current_path: Option<String> = None;
    let mut current_commit: Option<String> = None;
    let mut current_branch: Option<String> = None;

    for line in text.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            if let (Some(path), Some(commit)) = (&current_path, &current_commit) {
                entries.push(SourceWorktreeInventoryEntry {
                    path: path.clone(),
                    commit: commit.clone(),
                    branch: current_branch.clone(),
                    state: if path.starts_with('/') {
                        SourceWorktreeInventoryEntryState::Active
                    } else {
                        SourceWorktreeInventoryEntryState::Detached
                    },
                    path_basis: SourceWorktreeInventoryPathBasis::Absolute,
                    evidence_digest: String::new(),
                });
            }
            current_path = Some(path.to_string());
            current_commit = None;
            current_branch = None;
        } else if let Some(commit) = line.strip_prefix("HEAD ") {
            current_commit = Some(commit.to_string());
        } else if let Some(branch) = line.strip_prefix("branch ") {
            current_branch = Some(branch.to_string());
        } else if line.is_empty() {
            if let (Some(path), Some(commit)) = (&current_path, &current_commit) {
                entries.push(SourceWorktreeInventoryEntry {
                    path: path.clone(),
                    commit: commit.clone(),
                    branch: current_branch.clone(),
                    state: if path.starts_with('/') {
                        SourceWorktreeInventoryEntryState::Active
                    } else {
                        SourceWorktreeInventoryEntryState::Detached
                    },
                    path_basis: SourceWorktreeInventoryPathBasis::Absolute,
                    evidence_digest: String::new(),
                });
            }
            current_path = None;
            current_commit = None;
            current_branch = None;
        }
    }

    if let (Some(path), Some(commit)) = (&current_path, &current_commit) {
        entries.push(SourceWorktreeInventoryEntry {
            path: path.clone(),
            commit: commit.clone(),
            branch: current_branch.clone(),
            state: if path.starts_with('/') {
                SourceWorktreeInventoryEntryState::Active
            } else {
                SourceWorktreeInventoryEntryState::Detached
            },
            path_basis: SourceWorktreeInventoryPathBasis::Absolute,
            evidence_digest: String::new(),
        });
    }

    entries
}
