use std::collections::BTreeSet;

use cynic_node_protocol::digest_json;
use cynic_protocol::vcs::source_worktree_lease_target;

use super::workspace_types::{
    AGENT_WORKSPACE_REQUEST_SCHEMA, AgentWorkspaceChange, AgentWorkspaceClaimObservation,
    AgentWorkspaceRequest, AgentWorkspaceViolation, AgentWorkspaceZone, AgentWorkspaceZoneKind,
};

pub fn agent_workspace_target(repository_id: &str, workspace_id: &str) -> String {
    source_worktree_lease_target(repository_id, workspace_id)
}

pub fn zones_overlap(left: &AgentWorkspaceZone, right: &AgentWorkspaceZone) -> bool {
    let Some(left_path) = normalize_zone_path(&left.path) else {
        return false;
    };
    let Some(right_path) = normalize_zone_path(&right.path) else {
        return false;
    };
    if left_path == right_path {
        return true;
    }
    match (left.kind, right.kind) {
        (AgentWorkspaceZoneKind::Exact, AgentWorkspaceZoneKind::Exact) => false,
        (AgentWorkspaceZoneKind::Tree, AgentWorkspaceZoneKind::Exact) => {
            path_contains(&left_path, &right_path)
        }
        (AgentWorkspaceZoneKind::Exact, AgentWorkspaceZoneKind::Tree) => {
            path_contains(&right_path, &left_path)
        }
        (AgentWorkspaceZoneKind::Tree, AgentWorkspaceZoneKind::Tree) => {
            path_contains(&left_path, &right_path) || path_contains(&right_path, &left_path)
        }
    }
}

pub(super) fn validate_request(request: &AgentWorkspaceRequest) -> Vec<AgentWorkspaceViolation> {
    let mut violations = Vec::new();
    if request.schema != AGENT_WORKSPACE_REQUEST_SCHEMA {
        violations.push(violation(
            "invalid_workspace_request_schema",
            "workspace request schema is not supported",
        ));
    }
    for (field, value) in [
        ("workload_id", request.workload_id.as_str()),
        ("repository_id", request.repository_id.as_str()),
        ("workspace_id", request.workspace_id.as_str()),
        ("node_id", request.node_id.as_str()),
        ("agent_id", request.agent_id.as_str()),
        ("base_commit", request.base_commit.as_str()),
    ] {
        if value.trim().is_empty() {
            violations.push(violation(
                "empty_workspace_request_field",
                format!("{field} must not be empty"),
            ));
        }
    }
    if request.repository_id.contains('/')
        || request.repository_id.contains('\\')
        || request.workspace_id.contains('/')
        || request.workspace_id.contains('\\')
        || request.node_id.contains('/')
        || request.node_id.contains('\\')
    {
        violations.push(violation(
            "workspace_identity_not_path_safe",
            "repository, workspace, and node identifiers must not contain path separators",
        ));
    }
    if !valid_sha256_digest(&request.change_set_digest) {
        violations.push(violation(
            "invalid_change_set_digest",
            "change_set_digest must be a sha256 content digest",
        ));
    }
    match digest_json(&request.changes) {
        Ok(digest) if digest == request.change_set_digest => {}
        Ok(_) => violations.push(violation(
            "workspace_change_set_digest_mismatch",
            "change_set_digest must bind the exact ordered source changes",
        )),
        Err(_) => violations.push(violation(
            "workspace_change_set_not_serializable",
            "source changes could not be canonicalized",
        )),
    }
    if request.changes.is_empty() {
        violations.push(violation(
            "workspace_changes_missing",
            "a mutating workspace request requires at least one source change",
        ));
    }
    violations.extend(validate_zone_set(&request.zones));
    let mut changed_paths = BTreeSet::new();
    for change in &request.changes {
        validate_change(change, &request.zones, &mut changed_paths, &mut violations);
    }
    let expected_target = agent_workspace_target(&request.repository_id, &request.workspace_id);
    if request.guard.agent_id != request.agent_id
        || request.guard.target != expected_target
        || request.guard.lease_id.trim().is_empty()
        || request.guard.fencing_token == 0
    {
        violations.push(violation(
            "workspace_guard_mismatch",
            "the lease guard must bind the requesting agent and exact workspace target",
        ));
    }
    violations
}

pub(super) fn validate_zone_set(zones: &[AgentWorkspaceZone]) -> Vec<AgentWorkspaceViolation> {
    let mut violations = Vec::new();
    if zones.is_empty() {
        violations.push(violation(
            "workspace_zones_missing",
            "a mutating workspace operation requires at least one explicit zone",
        ));
    }
    for zone in zones {
        if normalize_zone_path(&zone.path).is_none() {
            violations.push(violation(
                "invalid_workspace_zone",
                format!("workspace zone '{}' is not repository-relative", zone.path),
            ));
        }
    }
    for (index, zone) in zones.iter().enumerate() {
        if zones[index + 1..]
            .iter()
            .any(|candidate| zones_overlap(zone, candidate))
        {
            violations.push(violation(
                "overlapping_requested_zones",
                "a request must use a minimal non-overlapping zone set",
            ));
        }
    }
    violations
}

fn validate_change(
    change: &AgentWorkspaceChange,
    zones: &[AgentWorkspaceZone],
    changed_paths: &mut BTreeSet<String>,
    violations: &mut Vec<AgentWorkspaceViolation>,
) {
    if normalize_zone_path(&change.path).as_deref() != Some(change.path.as_str()) {
        violations.push(violation(
            "invalid_workspace_change_path",
            format!("change path '{}' is not canonical", change.path),
        ));
    }
    if !changed_paths.insert(change.path.clone()) {
        violations.push(violation(
            "duplicate_workspace_change_path",
            format!("change path '{}' appears more than once", change.path),
        ));
    }
    if change.preimage_digest.is_none() && change.result_digest.is_none() {
        violations.push(violation(
            "empty_workspace_change",
            format!(
                "change path '{}' has neither preimage nor result",
                change.path
            ),
        ));
    }
    for (label, digest) in [
        ("preimage", change.preimage_digest.as_deref()),
        ("result", change.result_digest.as_deref()),
    ] {
        if digest.is_some_and(|value| !valid_sha256_digest(value)) {
            violations.push(violation(
                "invalid_workspace_change_digest",
                format!("{label} digest for '{}' is not sha256", change.path),
            ));
        }
    }
    if change.preimage_digest == change.result_digest {
        violations.push(violation(
            "workspace_change_is_noop",
            format!(
                "change path '{}' has identical before and after state",
                change.path
            ),
        ));
    }
    let covered = zones.iter().any(|zone| {
        let change_zone = AgentWorkspaceZone {
            kind: AgentWorkspaceZoneKind::Exact,
            path: change.path.clone(),
        };
        zones_overlap(zone, &change_zone)
    });
    if !covered {
        violations.push(violation(
            "workspace_change_outside_claimed_zones",
            format!(
                "change path '{}' is outside all requested zones",
                change.path
            ),
        ));
    }
}

pub(super) fn own_claim(
    request: &AgentWorkspaceRequest,
    claim: &&AgentWorkspaceClaimObservation,
) -> bool {
    claim.workload_id == request.workload_id
        && claim.workspace_id == request.workspace_id
        && claim.node_id == request.node_id
        && claim.agent_id == request.agent_id
        && claim.guard == request.guard
}

pub(super) fn normalized_zone_set(zones: &[AgentWorkspaceZone]) -> BTreeSet<AgentWorkspaceZone> {
    zones
        .iter()
        .filter_map(|zone| {
            normalize_zone_path(&zone.path).map(|path| AgentWorkspaceZone {
                kind: zone.kind,
                path,
            })
        })
        .collect()
}

fn normalize_zone_path(path: &str) -> Option<String> {
    let normalized = path.trim_end_matches('/');
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains('\\')
        || normalized
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return None;
    }
    Some(normalized.to_string())
}

fn path_contains(parent: &str, candidate: &str) -> bool {
    candidate
        .strip_prefix(parent)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

fn valid_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

pub(super) fn violation(
    code: impl Into<String>,
    message: impl Into<String>,
) -> AgentWorkspaceViolation {
    AgentWorkspaceViolation {
        code: code.into(),
        message: message.into(),
    }
}
