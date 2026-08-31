use std::path::{Component, Path, PathBuf};

use cynic_protocol::{
    SOURCE_WORKTREE_INVENTORY_MAX_ENTRIES, SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES,
    SourceWorktreeInventoryEntry, SourceWorktreeInventoryEntryState,
    SourceWorktreeInventoryPathBasis,
};

fn parse_porcelain(
    text: &str,
    repository_root: &Path,
    workspace_store: &Path,
) -> (Vec<SourceWorktreeInventoryEntry>, bool, Vec<String>) {
    if !text.is_empty() && !text.contains('\0') {
        return (
            Vec::new(),
            false,
            vec![cynic_protocol::digest::digest_bytes(text.as_bytes())],
        );
    }
    let (complete_text, trailing_evidence) = if text.is_empty() || text.ends_with("\0\0") {
        (text, Vec::new())
    } else if let Some(boundary) = text.rfind("\0\0") {
        let complete_end = boundary + 2;
        (
            &text[..complete_end],
            vec![cynic_protocol::digest::digest_bytes(
                text[complete_end..].as_bytes(),
            )],
        )
    } else {
        return (
            Vec::new(),
            false,
            vec![cynic_protocol::digest::digest_bytes(text.as_bytes())],
        );
    };
    let mut entries = Vec::new();
    let mut entry_limit_exceeded = false;
    let mut malformed_record_attributes: Vec<String> = Vec::new();

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
    let mut seen_branch = false;
    let mut seen_locked = false;
    let mut seen_bare = false;
    let mut seen_detached = false;
    let mut seen_prunable = false;

    fn mark_malformed_attribute(
        line: &str,
        unknown_attribute_count: &mut u16,
        unknown_attributes: &mut Vec<String>,
        malformed: &mut bool,
    ) {
        unknown_attributes.push(line.to_string());
        *unknown_attribute_count = unknown_attribute_count.saturating_add(1);
        *malformed = true;
    }

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
        let path_oversized = path.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES;
        let (layout_state, path_basis, relative_path) = if path_oversized {
            (
                SourceWorktreeInventoryEntryState::Malformed,
                SourceWorktreeInventoryPathBasis::Unknown,
                None,
            )
        } else if observed_path == repository_root {
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
        let effective_malformed = malformed || path_oversized;
        let state = if effective_malformed {
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
            "malformed": effective_malformed,
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

    let attributes = complete_text.split('\0').collect::<Vec<_>>();
    for line in attributes {
        if line.is_empty() {
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
            current_path = None;
            current_head = None;
            current_branch = None;
            current_locked = None;
            current_lock_reason = None;
            current_bare = None;
            current_detached = None;
            current_prunable = None;
            current_prunable_reason = None;
            current_unknown_attribute_count = 0;
            current_unknown_attributes.clear();
            current_malformed = false;
            seen_head = false;
            seen_branch = false;
            seen_locked = false;
            seen_bare = false;
            seen_detached = false;
            seen_prunable = false;
            continue;
        }
        if current_path.is_none() && !line.starts_with("worktree ") {
            malformed_record_attributes.push(line.to_string());
            continue;
        }
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
            seen_branch = false;
            seen_locked = false;
            seen_bare = false;
            seen_detached = false;
            seen_prunable = false;
        } else if line.starts_with("HEAD ") {
            let value = &line["HEAD ".len()..];
            if seen_head {
                current_head = None;
                mark_malformed_attribute(
                    line,
                    &mut current_unknown_attribute_count,
                    &mut current_unknown_attributes,
                    &mut current_malformed,
                );
            } else if value.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES {
                current_head = None;
                mark_malformed_attribute(
                    line,
                    &mut current_unknown_attribute_count,
                    &mut current_unknown_attributes,
                    &mut current_malformed,
                );
                seen_head = true;
            } else {
                current_head = Some(value.to_string());
                seen_head = true;
            }
        } else if line.starts_with("branch ") {
            let value = &line["branch ".len()..];
            if seen_branch {
                current_branch = None;
                mark_malformed_attribute(
                    line,
                    &mut current_unknown_attribute_count,
                    &mut current_unknown_attributes,
                    &mut current_malformed,
                );
            } else if value.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES {
                current_branch = None;
                mark_malformed_attribute(
                    line,
                    &mut current_unknown_attribute_count,
                    &mut current_unknown_attributes,
                    &mut current_malformed,
                );
                seen_branch = true;
            } else {
                current_branch = Some(value.to_string());
                seen_branch = true;
            }
        } else if line == "bare" {
            if seen_bare {
                current_bare = None;
                mark_malformed_attribute(
                    line,
                    &mut current_unknown_attribute_count,
                    &mut current_unknown_attributes,
                    &mut current_malformed,
                );
            } else {
                current_bare = Some(true);
                seen_bare = true;
            }
        } else if line == "locked" || line.starts_with("locked ") {
            let reason = line.strip_prefix("locked ");
            if seen_locked {
                current_locked = None;
                current_lock_reason = None;
                mark_malformed_attribute(
                    line,
                    &mut current_unknown_attribute_count,
                    &mut current_unknown_attributes,
                    &mut current_malformed,
                );
            } else if reason
                .is_some_and(|value| value.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES)
            {
                current_locked = Some(true);
                current_lock_reason = None;
                mark_malformed_attribute(
                    line,
                    &mut current_unknown_attribute_count,
                    &mut current_unknown_attributes,
                    &mut current_malformed,
                );
                seen_locked = true;
            } else {
                current_locked = Some(true);
                current_lock_reason = reason.map(ToString::to_string);
                seen_locked = true;
            }
        } else if line == "detached" {
            if seen_detached {
                current_detached = None;
                mark_malformed_attribute(
                    line,
                    &mut current_unknown_attribute_count,
                    &mut current_unknown_attributes,
                    &mut current_malformed,
                );
            } else {
                current_detached = Some(true);
                seen_detached = true;
            }
        } else if line == "prunable" || line.starts_with("prunable ") {
            let reason = line.strip_prefix("prunable ");
            if seen_prunable {
                current_prunable = None;
                current_prunable_reason = None;
                mark_malformed_attribute(
                    line,
                    &mut current_unknown_attribute_count,
                    &mut current_unknown_attributes,
                    &mut current_malformed,
                );
            } else if reason
                .is_some_and(|value| value.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES)
            {
                current_prunable = Some(true);
                current_prunable_reason = None;
                mark_malformed_attribute(
                    line,
                    &mut current_unknown_attribute_count,
                    &mut current_unknown_attributes,
                    &mut current_malformed,
                );
                seen_prunable = true;
            } else {
                current_prunable = Some(true);
                current_prunable_reason = reason.map(ToString::to_string);
                seen_prunable = true;
            }
        } else {
            current_unknown_attributes.push(line.to_string());
            if current_unknown_attribute_count == u16::MAX {
                current_malformed = true;
            } else {
                current_unknown_attribute_count += 1;
            }
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

    let mut malformed_record_evidence = trailing_evidence;
    if !malformed_record_attributes.is_empty() {
        malformed_record_evidence.push(
            cynic_protocol::digest::digest_json(&malformed_record_attributes).unwrap_or_default(),
        );
    }

    (entries, entry_limit_exceeded, malformed_record_evidence)
}
