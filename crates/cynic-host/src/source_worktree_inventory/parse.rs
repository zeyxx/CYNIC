use std::path::Path;

use cynic_protocol::{
    SOURCE_WORKTREE_INVENTORY_MAX_ENTRIES, SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES,
    SourceWorktreeInventoryEntry,
};

use super::entry::flush;

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

pub(crate) fn parse_porcelain(
    text: &str,
    repository_root: &Path,
    workspace_store: &Path,
) -> (Vec<SourceWorktreeInventoryEntry>, bool, Vec<String>) {
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
