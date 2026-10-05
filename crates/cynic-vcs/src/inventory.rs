//! Source worktree inventory — pure observation types, parser, validator and
//! the port-driven inventory observer.
//!
//! Zero direct I/O: `git worktree list --porcelain` bytes arrive through the
//! [`VcsRunner`] port; everything here is text processing, schema bounds and the
//! coherence validator every consumer runs.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use crate::ports::VcsRunner;
use crate::types::*;

const MAX_EVIDENCE_REFS: usize = 8192;
const MAX_ENTRY_UNKNOWNS: usize = 14;
const MAX_OBSERVATION_UNKNOWNS: usize = 6;

pub fn validate_source_worktree_inventory_observation(
    observation: &SourceWorktreeInventoryObservation,
) -> Vec<String> {
    let mut violations = Vec::new();
    if observation.schema != SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA {
        violations.push("worktree_inventory_observation_schema_mismatch".to_string());
    }
    if invalid_pathless_id(&observation.repository_id) {
        violations.push("worktree_inventory_repository_id_invalid".to_string());
    }
    if invalid_pathless_id(&observation.node_id) {
        violations.push("worktree_inventory_node_id_invalid".to_string());
    }
    if observation.observed_at_ms == 0 {
        violations.push("worktree_inventory_observed_at_missing".to_string());
    }
    if observation.entries.len() > SOURCE_WORKTREE_INVENTORY_MAX_ENTRIES {
        violations.push("worktree_inventory_entries_limit_exceeded".to_string());
    }
    if observation.evidence_refs.len() > MAX_EVIDENCE_REFS {
        violations.push("worktree_inventory_evidence_limit_exceeded".to_string());
    }
    if observation.repository_id.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES
        || observation.node_id.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES
    {
        violations.push("worktree_inventory_identity_size_exceeded".to_string());
    }
    if observation.entries.is_empty() && observation.observation_unknowns.is_empty() {
        violations.push("worktree_inventory_coverage_unknown_unannotated".to_string());
    }
    if observation.evidence_refs.is_empty() && observation.observation_unknowns.is_empty() {
        violations.push("worktree_inventory_evidence_missing".to_string());
    }
    if observation.observation_unknowns.len() > MAX_OBSERVATION_UNKNOWNS {
        violations.push("worktree_inventory_observation_unknown_limit_exceeded".to_string());
    }
    let mut observation_unknowns = HashSet::new();
    for unknown in observation
        .observation_unknowns
        .iter()
        .take(MAX_OBSERVATION_UNKNOWNS)
    {
        if !observation_unknowns.insert(*unknown) {
            violations.push("worktree_inventory_observation_unknown_duplicate".to_string());
        }
    }
    if !observation.entries.is_empty()
        && observation
            .observation_unknowns
            .iter()
            .take(MAX_OBSERVATION_UNKNOWNS)
            .any(|unknown| {
                matches!(
                    unknown,
                    SourceWorktreeInventoryObservationUnknown::GitCommandFailed
                        | SourceWorktreeInventoryObservationUnknown::EmptyOutput
                )
            })
    {
        violations.push("worktree_inventory_observation_unknown_contradictory".to_string());
    }
    for evidence_ref in observation.evidence_refs.iter().take(MAX_EVIDENCE_REFS) {
        if !valid_canonical_digest(evidence_ref) {
            violations.push("worktree_inventory_evidence_invalid".to_string());
        }
    }
    let evidence_refs = observation
        .evidence_refs
        .iter()
        .take(MAX_EVIDENCE_REFS)
        .collect::<HashSet<_>>();
    let mut candidate_ids = HashSet::new();
    for entry in observation
        .entries
        .iter()
        .take(SOURCE_WORKTREE_INVENTORY_MAX_ENTRIES)
    {
        if !candidate_ids.insert(&entry.candidate_id) {
            violations.push("worktree_inventory_candidate_id_duplicate".to_string());
        }
        if entry.candidate_id.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES
            || entry.evidence_digest.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES
            || entry
                .relative_path
                .as_ref()
                .is_some_and(|value| value.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES)
            || entry
                .head
                .as_ref()
                .is_some_and(|value| value.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES)
            || entry
                .observed_principal
                .as_ref()
                .is_some_and(|value| value.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES)
            || entry
                .branch
                .as_ref()
                .is_some_and(|value| value.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES)
            || entry
                .lock_reason
                .as_ref()
                .is_some_and(|value| value.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES)
            || entry
                .prunable_reason
                .as_ref()
                .is_some_and(|value| value.len() > SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES)
        {
            violations.push("worktree_inventory_entry_size_exceeded".to_string());
        }
        if !valid_canonical_digest(&entry.candidate_id)
            || !valid_canonical_digest(&entry.evidence_digest)
        {
            violations.push("worktree_inventory_entry_evidence_invalid".to_string());
        }
        if !evidence_refs.contains(&entry.evidence_digest) {
            violations.push("worktree_inventory_entry_evidence_unlinked".to_string());
        }
        if entry.unknowns.len() > MAX_ENTRY_UNKNOWNS {
            violations.push("worktree_inventory_unknown_limit_exceeded".to_string());
        }
        let inspected_unknown_count = entry.unknowns.len().min(MAX_ENTRY_UNKNOWNS);
        let entry_unknowns = entry
            .unknowns
            .iter()
            .take(MAX_ENTRY_UNKNOWNS)
            .copied()
            .collect::<HashSet<_>>();
        if entry_unknowns.len() != inspected_unknown_count {
            violations.push("worktree_inventory_unknown_duplicate".to_string());
        }
        if let Some(path) = entry.relative_path.as_deref() {
            if path.starts_with('/') || path.starts_with('\\') || is_drive_qualified(path) {
                violations.push("worktree_inventory_absolute_path_forbidden".to_string());
            }
            let components = path.split(['/', '\\']).collect::<Vec<_>>();
            if components.iter().any(|component| *component == "..") {
                violations.push("worktree_inventory_path_traversal_forbidden".to_string());
            }
            if path.is_empty()
                || components
                    .iter()
                    .any(|component| component.is_empty() || *component == ".")
                || path.chars().any(char::is_control)
                || components
                    .iter()
                    .any(|component| invalid_windows_component(component))
            {
                violations.push("worktree_inventory_relative_path_invalid".to_string());
            }
        }
        if entry.path_basis == SourceWorktreeInventoryPathBasis::Unknown
            && entry.relative_path.is_some()
        {
            violations.push("worktree_inventory_path_basis_conflict".to_string());
        }
        if entry.unknown_attribute_count > 0
            && !entry_unknowns.contains(&SourceWorktreeInventoryUnknown::GitAttribute)
        {
            violations.push("worktree_inventory_unknown_attribute_unannotated".to_string());
        }
        if entry.unknown_attribute_count == 0
            && entry_unknowns.contains(&SourceWorktreeInventoryUnknown::GitAttribute)
        {
            violations.push("worktree_inventory_unknown_attribute_contradictory".to_string());
        }
        if entry.lock_reason.is_some() && entry.locked != Some(true) {
            violations.push("worktree_inventory_lock_reason_conflict".to_string());
        }
        if entry.prunable_reason.is_some() && entry.git_prunable != Some(true) {
            violations.push("worktree_inventory_prunable_reason_conflict".to_string());
        }
        for (value, kind) in [
            (
                entry.relative_path.is_some()
                    || entry.path_basis == SourceWorktreeInventoryPathBasis::MainRepository,
                SourceWorktreeInventoryUnknown::RelativePath,
            ),
            (
                entry.head.is_some(),
                SourceWorktreeInventoryUnknown::GitHead,
            ),
            (
                entry.observed_principal.is_some(),
                SourceWorktreeInventoryUnknown::Principal,
            ),
            (
                entry.branch.is_some()
                    || entry.detached_head == Some(true)
                    || entry.bare == Some(true),
                SourceWorktreeInventoryUnknown::Branch,
            ),
            (
                entry.path_is_symlink.is_some(),
                SourceWorktreeInventoryUnknown::PathSymlink,
            ),
            (
                entry.git_registered.is_some(),
                SourceWorktreeInventoryUnknown::GitRegistration,
            ),
            (
                entry.git_prunable.is_some(),
                SourceWorktreeInventoryUnknown::GitPrunable,
            ),
            (
                entry.detached_head.is_some(),
                SourceWorktreeInventoryUnknown::DetachedHead,
            ),
            (
                entry.clean.is_some(),
                SourceWorktreeInventoryUnknown::Cleanliness,
            ),
            (
                entry.locked.is_some(),
                SourceWorktreeInventoryUnknown::Locked,
            ),
            (entry.bare.is_some(), SourceWorktreeInventoryUnknown::Bare),
        ] {
            let marked = entry_unknowns.contains(&kind);
            if value && marked {
                violations.push("worktree_inventory_unknown_contradictory".to_string());
            } else if !value && !marked {
                violations.push("worktree_inventory_unknown_unannotated".to_string());
            }
        }
        for (reason_is_known, parent_is_true, kind) in [
            (
                entry.lock_reason.is_some(),
                entry.locked == Some(true),
                SourceWorktreeInventoryUnknown::LockReason,
            ),
            (
                entry.prunable_reason.is_some(),
                entry.git_prunable == Some(true),
                SourceWorktreeInventoryUnknown::PrunableReason,
            ),
        ] {
            let marked = entry_unknowns.contains(&kind);
            if marked && (reason_is_known || !parent_is_true) {
                violations.push("worktree_inventory_unknown_contradictory".to_string());
            }
        }
    }
    violations.sort();
    violations.dedup();
    violations
}

fn invalid_pathless_id(value: &str) -> bool {
    value.trim().is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.chars().any(char::is_control)
}

fn is_drive_qualified(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn invalid_windows_component(value: &str) -> bool {
    value.ends_with(['.', ' '])
        || value
            .chars()
            .any(|character| matches!(character, ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        || matches!(
            value.to_ascii_uppercase().as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        )
}

fn valid_canonical_digest(value: &str) -> bool {
    cynic_protocol::digest::validate_digest(value, false).is_ok()
        && value
            .strip_prefix(cynic_protocol::digest::SHA3_PREFIX)
            .is_some_and(|body| {
                body.len() == 128
                    && body
                        .bytes()
                        .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
            })
}

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

pub fn parse_porcelain(
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
