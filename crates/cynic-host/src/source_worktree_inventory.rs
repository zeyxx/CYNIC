use std::path::Path;

use cynic_protocol::digest::digest_json;
use cynic_protocol::{
    SourceWorktreeInventoryEntry, SourceWorktreeInventoryEntryState,
    SourceWorktreeInventoryObservation, SourceWorktreeInventoryPathBasis,
    SourceWorktreeInventoryUnknown, validate_source_worktree_inventory_observation,
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
            schema: cynic_protocol::SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
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
        schema: cynic_protocol::SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
        repository_id: repository_id.to_string(),
        node_id: node_id.to_string(),
        observed_at_ms,
        entries,
        evidence_refs,
    }
}

fn parse_porcelain(text: &str, repository_root: &Path) -> Vec<SourceWorktreeInventoryEntry> {
    let mut entries = Vec::new();
    let root_canonical = repository_root.display().to_string();

    let mut current_path: Option<String> = None;
    let mut current_head: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut current_locked: Option<bool> = None;
    let mut current_lock_reason: Option<String> = None;
    let mut current_bare: Option<bool> = None;
    let mut current_detached: Option<bool> = None;
    let mut current_prunable: Option<bool> = None;
    let mut current_prunable_reason: Option<String> = None;

    fn flush(
        entries: &mut Vec<SourceWorktreeInventoryEntry>,
        path: &Option<String>,
        head: &Option<String>,
        branch: &Option<String>,
        locked: &Option<bool>,
        lock_reason: &Option<String>,
        bare: &Option<bool>,
        detached: &Option<bool>,
        prunable: &Option<bool>,
        prunable_reason: &Option<String>,
        root_canonical: &str,
    ) {
        let Some(path) = path else {
            return;
        };

        let path_basis = if path == root_canonical {
            SourceWorktreeInventoryPathBasis::MainRepository
        } else {
            SourceWorktreeInventoryPathBasis::WorkspaceLayout
        };

        let relative_path = if path_basis == SourceWorktreeInventoryPathBasis::MainRepository {
            None
        } else {
            Some(path.clone())
        };

        let mut unknowns: Vec<SourceWorktreeInventoryUnknown> = Vec::new();
        if head.is_none() {
            unknowns.push(SourceWorktreeInventoryUnknown::GitHead);
        }
        if branch.is_none() {
            unknowns.push(SourceWorktreeInventoryUnknown::Branch);
        }
        if locked.is_none() {
            unknowns.push(SourceWorktreeInventoryUnknown::Locked);
        }
        if bare.is_none() {
            unknowns.push(SourceWorktreeInventoryUnknown::Bare);
        }
        if detached.is_none() {
            unknowns.push(SourceWorktreeInventoryUnknown::DetachedHead);
        }
        if prunable.is_none() {
            unknowns.push(SourceWorktreeInventoryUnknown::GitPrunable);
        }

        let evidence_digest = cynic_protocol::digest_json(&(
            path,
            head,
            branch,
            locked,
            lock_reason,
            bare,
            detached,
            prunable,
            prunable_reason,
        ))
        .unwrap_or_default();

        entries.push(SourceWorktreeInventoryEntry {
            candidate_id: evidence_digest.clone(),
            evidence_digest,
            state: SourceWorktreeInventoryEntryState::Observed,
            unknown_attribute_count: 0,
            path_basis,
            relative_path,
            head: head.clone(),
            branch: branch.clone(),
            observed_principal: None,
            path_is_symlink: None,
            git_registered: Some(true),
            git_prunable: prunable.clone(),
            detached_head: detached.clone(),
            clean: None,
            locked: locked.clone(),
            bare: bare.clone(),
            lock_reason: lock_reason.clone(),
            prunable_reason: prunable_reason.clone(),
            unknowns,
        });
    }

    for line in text.lines() {
        if line.starts_with("worktree ") {
            flush(
                &mut entries,
                &current_path,
                &current_head,
                &current_branch,
                &current_locked,
                &current_lock_reason,
                &current_bare,
                &current_detached,
                &current_prunable,
                &current_prunable_reason,
                &root_canonical,
            );
            current_path = Some(line["worktree ".len()..].to_string());
            current_head = None;
            current_branch = None;
            current_locked = None;
            current_lock_reason = None;
            current_bare = None;
            current_detached = None;
            current_prunable = None;
            current_prunable_reason = None;
        } else if line.starts_with("HEAD ") {
            current_head = Some(line["HEAD ".len()..].to_string());
        } else if line.starts_with("branch ") {
            current_branch = Some(line["branch ".len()..].to_string());
        } else if line == "bare" {
            current_bare = Some(true);
        } else if line == "locked" {
            current_locked = Some(true);
        } else if line.starts_with("locked ") {
            current_locked = Some(true);
            current_lock_reason = Some(line["locked ".len()..].to_string());
        } else if line == "detached" {
            current_detached = Some(true);
        } else if line == "prunable" {
            current_prunable = Some(true);
        } else if line.starts_with("prunable ") {
            current_prunable = Some(true);
            current_prunable_reason = Some(line["prunable ".len()..].to_string());
        }
    }

    flush(
        &mut entries,
        &current_path,
        &current_head,
        &current_branch,
        &current_locked,
        &current_lock_reason,
        &current_bare,
        &current_detached,
        &current_prunable,
        &current_prunable_reason,
        &root_canonical,
    );

    entries
}

#[cfg(test)]
mod w1_b_tests {
    use super::*;
    use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};

    fn sha3(digest: &str) -> String {
        format!("sha3-512:{digest}")
    }

    fn porcelain_output() -> String {
        let main = sha3(&"a".repeat(128));
        let agent = sha3(&"b".repeat(128));
        format!(
            "worktree /var/lib/cynic/source/repositories/CYNIC\nHEAD {main}\nbranch refs/heads/main\n\nworktree /var/lib/cynic/source/workspaces/cynic/ws-a\nHEAD {agent}\nbranch refs/heads/agent/ws-a\nlocked agent running\n"
        )
    }

    #[test]
    fn adapter_produces_observation_from_porcelain_output() {
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: porcelain_output(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };
        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            "cynic",
            "builder-a",
            &runner,
        );
        assert_eq!(observation.repository_id, "cynic");
        assert_eq!(observation.node_id, "builder-a");
        assert_eq!(observation.entries.len(), 2);
        assert!(observation.evidence_refs.len() >= 2);
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn adapter_marks_main_and_workspace_path_basis() {
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: porcelain_output(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };
        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            "cynic",
            "builder-a",
            &runner,
        );
        assert_eq!(
            observation.entries[0].path_basis,
            SourceWorktreeInventoryPathBasis::MainRepository
        );
        assert_eq!(
            observation.entries[1].path_basis,
            SourceWorktreeInventoryPathBasis::WorkspaceLayout
        );
    }

    #[test]
    fn adapter_records_locked_fact() {
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: porcelain_output(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };
        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            "cynic",
            "builder-a",
            &runner,
        );
        assert_eq!(observation.entries[1].locked, Some(true));
        assert_eq!(
            observation.entries[1].lock_reason,
            Some("agent running".to_string())
        );
    }

    #[test]
    fn adapter_returns_empty_when_git_fails() {
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(128),
                    stdout: String::new(),
                    stderr: "fatal: not a git repository".to_string(),
                    spawn_error: None,
                },
            )],
        };
        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            "cynic",
            "builder-a",
            &runner,
        );
        assert_eq!(observation.entries.len(), 0);
        assert!(observation.evidence_refs.is_empty());
    }

    #[test]
    fn adapter_handles_empty_repository_with_no_worktrees() {
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: String::new(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };
        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            "cynic",
            "builder-a",
            &runner,
        );
        assert_eq!(observation.entries.len(), 0);
    }
}
