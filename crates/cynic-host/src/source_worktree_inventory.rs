use std::path::Path;

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
    let argv = ["git", "-C", &root, "worktree", "list", "--porcelain", "-z"];
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
    let mut workspace_store: Option<String> = None;

    let mut current_path: Option<String> = None;
    let mut current_head: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut current_locked: Option<bool> = None;
    let mut current_lock_reason: Option<String> = None;
    let mut current_bare: Option<bool> = None;
    let mut current_detached: Option<bool> = None;
    let mut current_prunable: Option<bool> = None;
    let mut current_prunable_reason: Option<String> = None;
    let mut current_unknown_attribute_count = 0_u16;

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
        unknown_attribute_count: u16,
        root_canonical: &str,
        workspace_store: &mut Option<String>,
    ) {
        let Some(path) = path else {
            return;
        };

        let path_basis = if path == root_canonical {
            SourceWorktreeInventoryPathBasis::MainRepository
        } else {
            if workspace_store.is_none() {
                if let Some(parent) = std::path::Path::new(path).parent() {
                    workspace_store.replace(parent.display().to_string());
                }
            }
            SourceWorktreeInventoryPathBasis::WorkspaceLayout
        };

        let relative_path = if path_basis == SourceWorktreeInventoryPathBasis::MainRepository {
            None
        } else if let Some(store) = workspace_store {
            std::path::Path::new(path)
                .strip_prefix(store)
                .ok()
                .map(|p| p.display().to_string().trim_start_matches('/').to_string())
        } else {
            std::path::Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
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
        if unknown_attribute_count > 0 {
            unknowns.push(SourceWorktreeInventoryUnknown::GitAttribute);
        }
        // Principal, path_is_symlink, clean are not observable from
        // `git worktree list --porcelain`; mark unknown.
        unknowns.push(SourceWorktreeInventoryUnknown::Principal);
        unknowns.push(SourceWorktreeInventoryUnknown::PathSymlink);
        unknowns.push(SourceWorktreeInventoryUnknown::Cleanliness);

        let evidence_digest = cynic_protocol::digest::digest_json(&serde_json::json!({
            "path": path,
            "head": head,
            "branch": branch,
            "locked": locked,
            "lock_reason": lock_reason,
            "bare": bare,
            "detached": detached,
            "prunable": prunable,
            "prunable_reason": prunable_reason,
            "unknown_attribute_count": unknown_attribute_count,
        }))
        .unwrap_or_default();

        entries.push(SourceWorktreeInventoryEntry {
            candidate_id: evidence_digest.clone(),
            evidence_digest,
            state: SourceWorktreeInventoryEntryState::Observed,
            unknown_attribute_count,
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

    let attributes = if text.contains('\0') {
        text.split('\0').collect::<Vec<_>>()
    } else {
        text.lines().collect::<Vec<_>>()
    };
    for line in attributes {
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
                current_unknown_attribute_count,
                &root_canonical,
                &mut workspace_store,
            );
            current_path = Some(line["worktree ".len()..].to_string());
            current_head = None;
            current_branch = None;
            current_locked = Some(false);
            current_lock_reason = None;
            current_bare = Some(false);
            current_detached = Some(false);
            current_prunable = Some(false);
            current_prunable_reason = None;
            current_unknown_attribute_count = 0;
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
        } else if !line.is_empty() {
            current_unknown_attribute_count = current_unknown_attribute_count.saturating_add(1);
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
        current_unknown_attribute_count,
        &root_canonical,
        &mut workspace_store,
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
                    "-z".to_string(),
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
                    "-z".to_string(),
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
    fn adapter_treats_absent_porcelain_flags_as_false() {
        let entries = parse_porcelain(
            &porcelain_output(),
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
        );

        assert_eq!(entries[0].locked, Some(false));
        assert_eq!(entries[0].bare, Some(false));
        assert_eq!(entries[0].detached_head, Some(false));
        assert_eq!(entries[0].git_prunable, Some(false));
        assert_eq!(entries[1].locked, Some(true));
        assert_eq!(entries[1].bare, Some(false));
        assert_eq!(entries[1].detached_head, Some(false));
        assert_eq!(entries[1].git_prunable, Some(false));
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
                    "-z".to_string(),
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
    fn adapter_preserves_nul_delimited_porcelain_values() {
        let head = sha3(&"a".repeat(128));
        let stdout = format!(
            "worktree /var/lib/cynic/source/repositories/CYNIC\0HEAD {head}\0branch refs/heads/main\0locked first line\nsecond line\0\0"
        );
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout,
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

        assert_eq!(observation.entries.len(), 1);
        assert_eq!(
            observation.entries[0].lock_reason,
            Some("first line\nsecond line".to_string())
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn adapter_preserves_unknown_porcelain_attributes() {
        let text = format!("{}future-attribute opaque-value\n", porcelain_output());
        let entries = parse_porcelain(&text, Path::new("/var/lib/cynic/source/repositories/CYNIC"));
        let evidence_refs = entries
            .iter()
            .map(|entry| entry.evidence_digest.clone())
            .collect();
        let observation = SourceWorktreeInventoryObservation {
            schema: cynic_protocol::SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
            repository_id: "cynic".to_string(),
            node_id: "builder-a".to_string(),
            observed_at_ms: 1,
            entries,
            evidence_refs,
        };

        assert_eq!(observation.entries[1].unknown_attribute_count, 1);
        assert!(
            observation.entries[1]
                .unknowns
                .contains(&SourceWorktreeInventoryUnknown::GitAttribute)
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
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
                    "-z".to_string(),
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
                    "-z".to_string(),
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
