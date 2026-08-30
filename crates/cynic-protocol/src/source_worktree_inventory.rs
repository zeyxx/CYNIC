use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA: &str =
    "cynic.source_worktree_inventory_observation.v2";
pub const SOURCE_WORKTREE_INVENTORY_MAX_ENTRIES: usize = 4096;
const MAX_EVIDENCE_REFS: usize = 8192;
const MAX_STRING_BYTES: usize = 4096;
const MAX_ENTRY_UNKNOWNS: usize = 14;
const MAX_OBSERVATION_UNKNOWNS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceWorktreeInventoryUnknown {
    Cleanliness,
    DetachedHead,
    GitHead,
    GitPrunable,
    GitRegistration,
    PathSymlink,
    RelativePath,
    Principal,
    Branch,
    Bare,
    Locked,
    LockReason,
    PrunableReason,
    GitAttribute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceWorktreeInventoryPathBasis {
    MainRepository,
    WorkspaceLayout,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceWorktreeInventoryEntryState {
    Observed,
    OutsideDeclaredLayout,
    Malformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceWorktreeInventoryObservationUnknown {
    GitCommandFailed,
    EmptyOutput,
    OutputLimitExceeded,
    EntryLimitExceeded,
    LayoutUnavailable,
    MalformedRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceWorktreeInventoryEntry {
    /// Opaque identity assigned by the adapter to one observed Git worktree.
    pub candidate_id: String,
    /// Canonical digest of the adapter evidence, pending independent verification.
    pub evidence_digest: String,
    pub state: SourceWorktreeInventoryEntryState,
    pub unknown_attribute_count: u16,
    pub path_basis: SourceWorktreeInventoryPathBasis,
    /// Relative to the declared `path_basis`; never an absolute machine path.
    #[serde(default)]
    pub relative_path: Option<String>,
    #[serde(default)]
    pub head: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub observed_principal: Option<String>,
    #[serde(default)]
    pub path_is_symlink: Option<bool>,
    #[serde(default)]
    pub git_registered: Option<bool>,
    #[serde(default)]
    pub git_prunable: Option<bool>,
    #[serde(default)]
    pub detached_head: Option<bool>,
    #[serde(default)]
    pub clean: Option<bool>,
    #[serde(default)]
    pub locked: Option<bool>,
    #[serde(default)]
    pub bare: Option<bool>,
    #[serde(default)]
    pub lock_reason: Option<String>,
    #[serde(default)]
    pub prunable_reason: Option<String>,
    #[serde(default)]
    pub unknowns: Vec<SourceWorktreeInventoryUnknown>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceWorktreeInventoryObservation {
    pub schema: String,
    pub repository_id: String,
    pub node_id: String,
    pub observed_at_ms: u64,
    #[serde(default)]
    pub entries: Vec<SourceWorktreeInventoryEntry>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub observation_unknowns: Vec<SourceWorktreeInventoryObservationUnknown>,
}

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
    if observation.repository_id.len() > MAX_STRING_BYTES
        || observation.node_id.len() > MAX_STRING_BYTES
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
        if entry.candidate_id.len() > MAX_STRING_BYTES
            || entry.evidence_digest.len() > MAX_STRING_BYTES
            || entry
                .relative_path
                .as_ref()
                .is_some_and(|value| value.len() > MAX_STRING_BYTES)
            || entry
                .head
                .as_ref()
                .is_some_and(|value| value.len() > MAX_STRING_BYTES)
            || entry
                .observed_principal
                .as_ref()
                .is_some_and(|value| value.len() > MAX_STRING_BYTES)
            || entry
                .branch
                .as_ref()
                .is_some_and(|value| value.len() > MAX_STRING_BYTES)
            || entry
                .lock_reason
                .as_ref()
                .is_some_and(|value| value.len() > MAX_STRING_BYTES)
            || entry
                .prunable_reason
                .as_ref()
                .is_some_and(|value| value.len() > MAX_STRING_BYTES)
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
    crate::digest::validate_digest(value, false).is_ok()
        && value
            .strip_prefix(crate::digest::SHA3_PREFIX)
            .is_some_and(|body| {
                body.len() == 128
                    && body
                        .bytes()
                        .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
            })
}
