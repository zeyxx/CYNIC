use std::path::{Component, Path, PathBuf};

use cynic_protocol::{
    SOURCE_WORKTREE_INVENTORY_MAX_ENTRIES, SourceWorktreeInventoryEntry,
    SourceWorktreeInventoryEntryState, SourceWorktreeInventoryObservation,
    SourceWorktreeInventoryObservationUnknown, SourceWorktreeInventoryPathBasis,
    SourceWorktreeInventoryUnknown, validate_source_worktree_inventory_observation,
};

use crate::update_lifecycle::ArgvRunner;

const MAX_PORCELAIN_BYTES: usize = 4 * 1024 * 1024;

pub fn observe_source_worktree_inventory(
    repository_root: &Path,
    workspace_store: &Path,
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
        return unknown_observation(
            repository_id,
            node_id,
            observed_at_ms,
            SourceWorktreeInventoryObservationUnknown::GitCommandFailed,
        );
    }

    let text = if output.stdout.is_empty() {
        output.stderr
    } else {
        output.stdout
    };
    if text.len() > MAX_PORCELAIN_BYTES {
        return unknown_observation(
            repository_id,
            node_id,
            observed_at_ms,
            SourceWorktreeInventoryObservationUnknown::OutputLimitExceeded,
        );
    }

    let repository_root = observed_absolute_path(repository_root);
    let workspace_store = observed_absolute_path(workspace_store);
    let (entries, entry_limit_exceeded) =
        parse_porcelain(&text, &repository_root, &workspace_store);
    let evidence_refs: Vec<String> = entries.iter().map(|e| e.evidence_digest.clone()).collect();
    let observation_unknowns = if entry_limit_exceeded {
        vec![SourceWorktreeInventoryObservationUnknown::EntryLimitExceeded]
    } else if entries.is_empty() {
        vec![SourceWorktreeInventoryObservationUnknown::EmptyOutput]
    } else {
        Vec::new()
    };

    SourceWorktreeInventoryObservation {
        schema: cynic_protocol::SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
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
        schema: cynic_protocol::SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
        repository_id: repository_id.to_string(),
        node_id: node_id.to_string(),
        observed_at_ms,
        entries: Vec::new(),
        evidence_refs: Vec::new(),
        observation_unknowns: vec![unknown],
    }
}

fn observed_absolute_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn parse_porcelain(
    text: &str,
    repository_root: &Path,
    workspace_store: &Path,
) -> (Vec<SourceWorktreeInventoryEntry>, bool) {
    let mut entries = Vec::new();
    let mut entry_limit_exceeded = false;

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
    let mut current_unknown_attributes: Vec<String> = Vec::new();
    let mut current_malformed = false;
    let mut seen_head = false;

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
        unknown_attributes: &[String],
        malformed: bool,
        repository_root: &Path,
        workspace_store: &Path,
        entry_limit_exceeded: &mut bool,
    ) {
        let Some(path) = path else {
            return;
        };
        if entries.len() >= SOURCE_WORKTREE_INVENTORY_MAX_ENTRIES {
            *entry_limit_exceeded = true;
            return;
        }

        let observed_path = Path::new(path);
        let (layout_state, path_basis, relative_path) = if observed_path == repository_root {
            (
                SourceWorktreeInventoryEntryState::Observed,
                SourceWorktreeInventoryPathBasis::MainRepository,
                None,
            )
        } else if let Ok(relative) = observed_path.strip_prefix(workspace_store) {
            if relative.as_os_str().is_empty()
                || !relative
                    .components()
                    .all(|component| matches!(component, Component::Normal(_)))
            {
                (
                    SourceWorktreeInventoryEntryState::OutsideDeclaredLayout,
                    SourceWorktreeInventoryPathBasis::Unknown,
                    None,
                )
            } else {
                (
                    SourceWorktreeInventoryEntryState::Observed,
                    SourceWorktreeInventoryPathBasis::WorkspaceLayout,
                    Some(relative.display().to_string()),
                )
            }
        } else {
            (
                SourceWorktreeInventoryEntryState::OutsideDeclaredLayout,
                SourceWorktreeInventoryPathBasis::Unknown,
                None,
            )
        };
        let state = if malformed {
            SourceWorktreeInventoryEntryState::Malformed
        } else {
            layout_state
        };

        let mut unknowns: Vec<SourceWorktreeInventoryUnknown> = Vec::new();
        if head.is_none() {
            unknowns.push(SourceWorktreeInventoryUnknown::GitHead);
        }
        if branch.is_none() && *detached != Some(true) && *bare != Some(true) {
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
        if path_basis == SourceWorktreeInventoryPathBasis::Unknown {
            unknowns.push(SourceWorktreeInventoryUnknown::RelativePath);
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
            "unknown_attributes": unknown_attributes,
            "malformed": malformed,
        }))
        .unwrap_or_default();

        entries.push(SourceWorktreeInventoryEntry {
            candidate_id: evidence_digest.clone(),
            evidence_digest,
            state,
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
                &current_unknown_attributes,
                current_malformed,
                repository_root,
                workspace_store,
                &mut entry_limit_exceeded,
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
            current_unknown_attributes.clear();
            current_malformed = false;
            seen_head = false;
        } else if line.starts_with("HEAD ") {
            if seen_head {
                current_head = None;
                current_unknown_attributes.push(line.to_string());
                current_unknown_attribute_count = current_unknown_attribute_count.saturating_add(1);
                current_malformed = true;
            } else {
                current_head = Some(line["HEAD ".len()..].to_string());
                seen_head = true;
            }
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
            current_unknown_attributes.push(line.to_string());
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
        &current_unknown_attributes,
        current_malformed,
        repository_root,
        workspace_store,
        &mut entry_limit_exceeded,
    );

    (entries, entry_limit_exceeded)
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
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
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
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
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
    fn parser_uses_injected_layout_and_marks_outside_paths() {
        let root = tempfile::tempdir().unwrap();
        let repository = root.path().join("repositories/CYNIC");
        let workspace_store = root.path().join("workspaces/cynic");
        let workspace = workspace_store.join("ws-a");
        let outside = root.path().join("foreign/ws-b");
        std::fs::create_dir_all(&repository).unwrap();
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let head = sha3(&"a".repeat(128));
        let stdout = format!(
            "worktree {}\0HEAD {head}\0branch refs/heads/main\0\0worktree {}\0HEAD {head}\0branch refs/heads/agent/ws-a\0\0worktree {}\0HEAD {head}\0branch refs/heads/foreign\0\0",
            repository.display(),
            workspace.display(),
            outside.display(),
        );

        let (entries, entry_limit_exceeded) =
            parse_porcelain(&stdout, &repository, &workspace_store);

        assert!(!entry_limit_exceeded);
        assert_eq!(
            entries[0].path_basis,
            SourceWorktreeInventoryPathBasis::MainRepository
        );
        assert_eq!(
            entries[0].state,
            SourceWorktreeInventoryEntryState::Observed
        );
        assert_eq!(
            entries[1].path_basis,
            SourceWorktreeInventoryPathBasis::WorkspaceLayout
        );
        assert_eq!(entries[1].relative_path.as_deref(), Some("ws-a"));
        assert_eq!(
            entries[1].state,
            SourceWorktreeInventoryEntryState::Observed
        );
        assert_eq!(
            entries[2].path_basis,
            SourceWorktreeInventoryPathBasis::Unknown
        );
        assert_eq!(entries[2].relative_path, None);
        assert_eq!(
            entries[2].state,
            SourceWorktreeInventoryEntryState::OutsideDeclaredLayout
        );
        assert!(
            entries[2]
                .unknowns
                .contains(&SourceWorktreeInventoryUnknown::RelativePath)
        );
    }

    #[test]
    fn adapter_treats_absent_porcelain_flags_as_false() {
        let (entries, entry_limit_exceeded) = parse_porcelain(
            &porcelain_output(),
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
        );

        assert!(!entry_limit_exceeded);
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
    fn detached_head_makes_branch_absence_known() {
        let head = sha3(&"a".repeat(128));
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
                    stdout: format!(
                        "worktree /var/lib/cynic/source/workspaces/cynic/ws-detached\0HEAD {head}\0detached\0\0"
                    ),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };

        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );

        assert_eq!(observation.entries[0].branch, None);
        assert_eq!(observation.entries[0].detached_head, Some(true));
        assert!(
            !observation.entries[0]
                .unknowns
                .contains(&SourceWorktreeInventoryUnknown::Branch)
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
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
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
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
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
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
        let (entries, entry_limit_exceeded) = parse_porcelain(
            &text,
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
        );
        assert!(!entry_limit_exceeded);
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
            observation_unknowns: Vec::new(),
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
    fn unknown_porcelain_content_changes_the_evidence_digest() {
        let head = sha3(&"a".repeat(128));
        let first = format!(
            "worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {head}\0future-alpha opaque-a\0\0"
        );
        let second = format!(
            "worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {head}\0future-beta opaque-b\0\0"
        );
        let repository = Path::new("/var/lib/cynic/source/repositories/CYNIC");
        let workspace_store = Path::new("/var/lib/cynic/source/workspaces/cynic");

        let (first_entries, first_limit) = parse_porcelain(&first, repository, workspace_store);
        let (second_entries, second_limit) = parse_porcelain(&second, repository, workspace_store);

        assert!(!first_limit);
        assert!(!second_limit);
        assert_eq!(first_entries[0].unknown_attribute_count, 1);
        assert_eq!(second_entries[0].unknown_attribute_count, 1);
        assert_ne!(
            first_entries[0].evidence_digest,
            second_entries[0].evidence_digest
        );
    }

    #[test]
    fn duplicate_known_attribute_marks_the_record_malformed() {
        let first_head = sha3(&"a".repeat(128));
        let second_head = sha3(&"b".repeat(128));
        let text = format!(
            "worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {first_head}\0HEAD {second_head}\0\0"
        );

        let (entries, entry_limit_exceeded) = parse_porcelain(
            &text,
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
        );
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
            observation_unknowns: Vec::new(),
        };

        assert!(!entry_limit_exceeded);
        assert_eq!(
            observation.entries[0].state,
            SourceWorktreeInventoryEntryState::Malformed
        );
        assert_eq!(observation.entries[0].head, None);
        assert_eq!(observation.entries[0].unknown_attribute_count, 1);
        assert!(
            observation.entries[0]
                .unknowns
                .contains(&SourceWorktreeInventoryUnknown::GitHead)
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn every_singleton_porcelain_field_rejects_duplicates() {
        let head = sha3(&"a".repeat(128));
        for duplicate_fields in [
            "branch refs/heads/one\0branch refs/heads/two",
            "bare\0bare",
            "locked\0locked reason",
            "detached\0detached",
            "prunable\0prunable reason",
        ] {
            let text = format!(
                "worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {head}\0{duplicate_fields}\0\0"
            );
            let (entries, entry_limit_exceeded) = parse_porcelain(
                &text,
                Path::new("/var/lib/cynic/source/repositories/CYNIC"),
                Path::new("/var/lib/cynic/source/workspaces/cynic"),
            );

            assert!(!entry_limit_exceeded);
            assert_eq!(
                entries[0].state,
                SourceWorktreeInventoryEntryState::Malformed,
                "duplicate fields: {duplicate_fields}"
            );
            assert_eq!(
                entries[0].unknown_attribute_count, 1,
                "duplicate fields: {duplicate_fields}"
            );
        }
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
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );
        assert_eq!(observation.entries.len(), 0);
        assert!(observation.evidence_refs.is_empty());
        assert_eq!(
            observation.observation_unknowns,
            vec![SourceWorktreeInventoryObservationUnknown::GitCommandFailed]
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn adapter_rejects_oversized_porcelain_before_parsing() {
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
                    stdout: "x".repeat(4 * 1024 * 1024 + 1),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };

        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );

        assert!(observation.entries.is_empty());
        assert_eq!(
            observation.observation_unknowns,
            vec![SourceWorktreeInventoryObservationUnknown::OutputLimitExceeded]
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn adapter_stops_at_the_entry_limit() {
        let head = sha3(&"a".repeat(128));
        let mut stdout = String::new();
        for index in 0..=4096 {
            stdout.push_str(&format!(
                "worktree /var/lib/cynic/source/workspaces/cynic/ws-{index}\0HEAD {head}\0branch refs/heads/agent/ws-{index}\0\0"
            ));
        }
        assert!(stdout.len() < MAX_PORCELAIN_BYTES);
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
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );

        assert_eq!(observation.entries.len(), 4096);
        assert_eq!(
            observation.observation_unknowns,
            vec![SourceWorktreeInventoryObservationUnknown::EntryLimitExceeded]
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
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
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );
        assert_eq!(observation.entries.len(), 0);
        assert_eq!(
            observation.observation_unknowns,
            vec![SourceWorktreeInventoryObservationUnknown::EmptyOutput]
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }
}
