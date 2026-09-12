use std::path::{Component, Path};

use cynic_protocol::{
    SOURCE_WORKTREE_INVENTORY_MAX_ENTRIES, SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES,
    SourceWorktreeInventoryEntry, SourceWorktreeInventoryEntryState,
    SourceWorktreeInventoryPathBasis, SourceWorktreeInventoryUnknown,
};

pub(crate) fn flush(
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
