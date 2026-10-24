use std::{fs, path::Path};

use serde::Deserialize;

use super::types::{
    BUILD_CACHE_PLAN_SCHEMA, BuildCacheDecision, BuildCacheDesiredState, BuildCacheObservation,
    BuildCachePathState, BuildCachePlan, EphemeralEntry, EphemeralKind, EphemeralScope,
};

#[derive(Deserialize)]
struct Registry {
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    id: Option<String>,
    name: Option<String>,
    role: Option<String>,
    build_lifecycle: Option<BuildLifecycle>,
}

/// The build cache path the registry declares, for the build runner.
///
/// WHY A RESOLVER AND NOT A CONSTANT. The runner used to compile the
/// build cache path into itself (`state_dir().join("shared-target")`)
/// while this registry declared another one (`build-targets/bootstrap`).
/// Two claims about the same directory on the same machine, and they
/// disagreed.
///
/// That divergence is not cosmetic. `cynic build reclaim` reads the
/// registry to know what to reclaim, and refuses to touch the canonical
/// profile when the canonical path does not exist:
///
/// ```text
/// canonical_target_state: "missing"
/// reclaim_profile_state: "missing"
/// ```
///
/// So the 30 Go of real build cache sat outside the governance of the
/// policy that exists to reclaim it, while the registry pointed at a
/// directory that nothing had ever created.
///
/// WHY THE REGISTRY WINS. `infra/directory-tree.json` is the authority
/// for the runtime tree, and it says the build target is
/// `{state-root}/build-targets/<profile>`. A path in the code and a path
/// in the registry that disagree are two claims about the same machine;
/// the registry is the one that is read by the tooling that has to act
/// on it.
///
/// The runner therefore asks the registry where the cache is, and a
/// registry that cannot answer is an explicit refusal — never a guess.
pub fn canonical_target_relative_path(registry_path: &Path) -> Result<String, String> {
    let bytes = fs::read(registry_path)
        .map_err(|error| format!("build_cache_registry_read_failed:{error}"))?;
    let registry: Registry = serde_json::from_slice(&bytes)
        .map_err(|error| format!("build_cache_registry_decode_failed:{error}"))?;
    let mut found: Vec<String> = Vec::new();
    for node in registry.nodes.iter() {
        if node.role.as_deref() != Some("build_runner") {
            continue;
        }
        let Some(policy) = node.build_lifecycle.as_ref() else {
            continue;
        };
        let path = policy.canonical_target_relative_path.trim();
        if path.is_empty() {
            continue;
        }
        if path.starts_with('/') {
            return Err("canonical_target_not_relative".to_string());
        }
        if path
            .split('/')
            .any(|component| component == ".." || component.is_empty())
        {
            return Err("canonical_target_escapes_state_root".to_string());
        }
        found.push(path.to_string());
    }
    found.sort();
    found.dedup();
    match found.as_slice() {
        [] => Err("build_runner_canonical_target_undeclared".to_string()),
        [path] => Ok(path.clone()),
        _ => Err("build_runner_canonical_target_ambiguous".to_string()),
    }
}

#[derive(Deserialize)]
struct BuildLifecycle {
    schema: String,
    minimum_available_bytes: u64,
    canonical_target_relative_path: String,
    canonical_cache_reclaim: CacheReclaim,
    noncanonical_cache_candidates: Vec<CacheCandidate>,
    /// Opt-in retention for ephemeral scopes. Absent in registries that
    /// predate it — absence disables retention and preserves behavior.
    #[serde(default)]
    ephemeral_retention: Option<EphemeralRetention>,
}

#[derive(Deserialize)]
struct EphemeralRetention {
    schema: String,
    max_age_ms: u64,
    pressure_grace_ms: u64,
    scopes: Vec<EphemeralScopeDecl>,
}

#[derive(Deserialize)]
struct EphemeralScopeDecl {
    base: String,
    relative_path: String,
    kind: String,
    #[serde(default)]
    workload_store_relative_path: Option<String>,
}

#[derive(Deserialize)]
struct CacheReclaim {
    schema: String,
    trigger: String,
    action: String,
    target_profile: String,
    preserve_profiles: Vec<String>,
    rollback_intent: String,
}

#[derive(Deserialize)]
struct CacheCandidate {
    base: String,
    relative_path: String,
}

pub fn load_build_cache_desired_state(
    registry_path: &Path,
    node_id: &str,
    state_root: &Path,
    temporary_root: &Path,
) -> Result<BuildCacheDesiredState, String> {
    let bytes = fs::read(registry_path)
        .map_err(|error| format!("build_cache_registry_read_failed:{error}"))?;
    let registry: Registry = serde_json::from_slice(&bytes)
        .map_err(|error| format!("build_cache_registry_decode_failed:{error}"))?;
    let mut nodes = registry.nodes.into_iter().filter(|node| {
        node.id.as_deref() == Some(node_id) || node.name.as_deref() == Some(node_id)
    });
    let node = nodes
        .next()
        .ok_or_else(|| "build_cache_node_missing".to_string())?;
    if nodes.next().is_some() || node.role.as_deref() != Some("build_runner") {
        return Err("build_cache_node_identity_invalid".to_string());
    }
    let policy = node
        .build_lifecycle
        .ok_or_else(|| "build_cache_policy_missing".to_string())?;
    validate_policy(&policy, state_root, temporary_root)?;
    let canonical_target = state_root.join(&policy.canonical_target_relative_path);
    let noncanonical_cache_paths = policy
        .noncanonical_cache_candidates
        .into_iter()
        .map(|candidate| match candidate.base.as_str() {
            "state_root" => Ok(state_root.join(candidate.relative_path)),
            "temporary_directory" => Ok(temporary_root.join(candidate.relative_path)),
            _ => Err("build_cache_candidate_base_unsupported".to_string()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let retention = &policy.ephemeral_retention;
    let mut ephemeral_scopes = Vec::new();
    if let Some(retention) = retention {
        for scope in &retention.scopes {
            let kind = match scope.kind.as_str() {
                "evidence_checkout" => EphemeralKind::EvidenceCheckout,
                "workload_workspace" => EphemeralKind::WorkloadWorkspace,
                _ => return Err("build_cache_retention_kind_unsupported".to_string()),
            };
            if scope.kind.as_str() == "workload_workspace"
                && scope.workload_store_relative_path.is_none()
            {
                return Err("build_cache_retention_workspace_store_missing".to_string());
            }
            ephemeral_scopes.push(EphemeralScope {
                base: scope.base.clone(),
                relative_path: scope.relative_path.clone(),
                kind,
                workload_store_relative_path: scope.workload_store_relative_path.clone(),
            });
        }
    }
    Ok(BuildCacheDesiredState {
        node_id: node_id.to_string(),
        state_root: path_string(state_root)?,
        temporary_root: path_string(temporary_root)?,
        canonical_target: path_string(&canonical_target)?,
        minimum_available_bytes: policy.minimum_available_bytes,
        reclaim_profile: policy.canonical_cache_reclaim.target_profile,
        preserved_profiles: policy.canonical_cache_reclaim.preserve_profiles,
        noncanonical_cache_paths: noncanonical_cache_paths
            .iter()
            .map(|path| path_string(path))
            .collect::<Result<Vec<_>, _>>()?,
        ephemeral_scopes,
        retention_max_age_ms: retention.as_ref().map(|retention| retention.max_age_ms),
        retention_pressure_grace_ms: retention
            .as_ref()
            .map(|retention| retention.pressure_grace_ms),
    })
}

pub fn reconcile_build_cache(
    desired: &BuildCacheDesiredState,
    observed: &BuildCacheObservation,
) -> BuildCachePlan {
    let mut blockers = observed.unknowns.clone();
    let decision = if observed.node_id != desired.node_id
        || observed.canonical_target != desired.canonical_target
    {
        blockers.push("build_cache_observation_identity_mismatch".to_string());
        BuildCacheDecision::Blocked
    } else if !ephemeral_eligible(desired, observed).is_empty() {
        // Dead weight goes before everything else, including the
        // non-canonical refusal: ephemeral entries are verified dead
        // (terminal workspaces) or retention-aged (checkouts past the hard
        // cap, or past the pressure grace under pressure), while a present
        // non-canonical cache may still be live. Reclaiming the dead first
        // is strictly safer than refusing on the ungoverned.
        BuildCacheDecision::ReclaimEphemeral
    } else if !observed.observed_noncanonical_cache_paths.is_empty() {
        blockers.push("noncanonical_build_cache_present".to_string());
        BuildCacheDecision::Blocked
    } else if matches!(
        observed.canonical_target_state,
        BuildCachePathState::Rejected | BuildCachePathState::Unknown
    ) || matches!(
        observed.reclaim_profile_state,
        BuildCachePathState::Rejected | BuildCachePathState::Unknown
    ) {
        blockers.push("build_cache_path_rejected_or_unknown".to_string());
        BuildCacheDecision::Blocked
    } else if !observed.unknowns.is_empty() {
        BuildCacheDecision::Blocked
    } else if observed
        .available_bytes
        .is_some_and(|bytes| bytes >= desired.minimum_available_bytes)
    {
        BuildCacheDecision::Satisfied
    } else if observed.available_bytes.is_none() {
        blockers.push("build_cache_available_bytes_unknown".to_string());
        BuildCacheDecision::Blocked
    } else if observed.canonical_target_state == BuildCachePathState::SafeDirectory
        && observed.reclaim_profile_state == BuildCachePathState::SafeDirectory
    {
        BuildCacheDecision::ReclaimCanonicalProfile
    } else {
        blockers.push("build_cache_reclaimable_profile_absent".to_string());
        BuildCacheDecision::Blocked
    };
    blockers.sort();
    blockers.dedup();
    let ephemeral_targets = match decision {
        BuildCacheDecision::ReclaimEphemeral => ephemeral_eligible(desired, observed)
            .into_iter()
            .map(|entry| entry.path)
            .collect(),
        _ => Vec::new(),
    };
    BuildCachePlan {
        schema: BUILD_CACHE_PLAN_SCHEMA,
        status: match decision {
            BuildCacheDecision::Satisfied => "satisfied",
            BuildCacheDecision::ReclaimEphemeral => "planned",
            BuildCacheDecision::ReclaimCanonicalProfile => "planned",
            BuildCacheDecision::Blocked => "blocked",
        },
        node_id: desired.node_id.clone(),
        mutates: matches!(
            decision,
            BuildCacheDecision::ReclaimEphemeral | BuildCacheDecision::ReclaimCanonicalProfile
        ),
        decision,
        target_profile: desired.reclaim_profile.clone(),
        target_path: observed.reclaim_profile_path.clone(),
        ephemeral_targets,
        preserved_profiles: desired.preserved_profiles.clone(),
        minimum_available_bytes: desired.minimum_available_bytes,
        observed_available_bytes: observed.available_bytes,
        blockers,
        rollback_intent: "none_reproducible_build_cache_only",
    }
}

/// Pure eligibility for ephemeral retention, eldest first.
///
/// Retention is opt-in: without a declared `ephemeral_retention` section
/// (both `Option`s `None`) nothing is ever eligible and the historical
/// decision chain runs unchanged. Liveness is fail-closed: a workspace is
/// eligible only on a verified terminal read-back; live or unclassifiable
/// entries are never eligible, whatever their age or the pressure.
fn ephemeral_eligible(
    desired: &BuildCacheDesiredState,
    observed: &BuildCacheObservation,
) -> Vec<EphemeralEntry> {
    let (Some(max_age_ms), Some(grace_ms)) = (
        desired.retention_max_age_ms,
        desired.retention_pressure_grace_ms,
    ) else {
        return Vec::new();
    };
    let pressure = observed
        .available_bytes
        .is_some_and(|bytes| bytes < desired.minimum_available_bytes);
    let mut eligible: Vec<EphemeralEntry> = observed
        .observed_ephemeral
        .iter()
        .filter(|entry| match entry.kind {
            EphemeralKind::EvidenceCheckout => {
                entry.age_ms > max_age_ms || (pressure && entry.age_ms > grace_ms)
            }
            EphemeralKind::WorkloadWorkspace => {
                entry.workload_terminal == Some(true) && (pressure || entry.age_ms > max_age_ms)
            }
        })
        .cloned()
        .collect();
    eligible.sort_by(|left, right| {
        right
            .age_ms
            .cmp(&left.age_ms)
            .then_with(|| left.path.cmp(&right.path))
    });
    eligible
}

fn validate_policy(
    policy: &BuildLifecycle,
    state_root: &Path,
    temporary_root: &Path,
) -> Result<(), String> {
    let reclaim = &policy.canonical_cache_reclaim;
    let preserves_release = reclaim.preserve_profiles.len() == 1
        && reclaim.preserve_profiles.first().map(String::as_str) == Some("release");
    if policy.schema != "cynic.build_runner.resource_policy.v1"
        || policy.minimum_available_bytes == 0
        || !safe_relative(&policy.canonical_target_relative_path)
        || reclaim.schema != "cynic.build_runner.cache_reclaim_policy.v1"
        || reclaim.trigger != "available_bytes_below_minimum"
        || reclaim.action != "rust_remove_bounded_profile"
        || reclaim.target_profile != "debug"
        || !preserves_release
        || reclaim.rollback_intent != "none_reproducible_build_cache_only"
        || !state_root.is_absolute()
        || !temporary_root.is_absolute()
        || policy.noncanonical_cache_candidates.is_empty()
        || policy
            .noncanonical_cache_candidates
            .iter()
            .any(|candidate| !safe_relative(&candidate.relative_path))
    {
        return Err("build_cache_policy_invalid".to_string());
    }
    if let Some(retention) = &policy.ephemeral_retention {
        validate_retention(retention)?;
    }
    Ok(())
}

/// Resolve the minimum free bytes a node requires before a checkout may be
/// materialized on it.
///
/// The value is declared in the registry under
/// `build_lifecycle.minimum_available_bytes`. It is the threshold the
/// pre-trigger disk gate enforces: below it, the build is refused with an
/// explicit verdict instead of materializing a checkout that cannot fit.
///
/// A node without a `build_lifecycle` block has no declared threshold, and
/// the gate stays off for it. Absent policy is not a silent zero: it is an
/// explicit `None`, and the caller decides whether the absence is a gap.
pub fn resolve_minimum_available_bytes(
    registry_path: &Path,
    node_id: &str,
) -> Result<Option<u64>, String> {
    let bytes = fs::read(registry_path)
        .map_err(|error| format!("build_cache_registry_read_failed:{error}"))?;
    let registry: Registry = serde_json::from_slice(&bytes)
        .map_err(|error| format!("build_cache_registry_decode_failed:{error}"))?;
    let node = registry
        .nodes
        .into_iter()
        .find(|node| node.id.as_deref() == Some(node_id) || node.name.as_deref() == Some(node_id))
        .ok_or_else(|| "build_cache_node_missing".to_string())?;
    Ok(node
        .build_lifecycle
        .map(|policy| policy.minimum_available_bytes))
}

fn validate_retention(retention: &EphemeralRetention) -> Result<(), String> {
    if retention.schema != "cynic.build_runner.ephemeral_retention.v1"
        || retention.max_age_ms == 0
        || retention.scopes.is_empty()
    {
        return Err("build_cache_retention_invalid".to_string());
    }
    for scope in &retention.scopes {
        let kind_ok = scope.kind == "evidence_checkout" || scope.kind == "workload_workspace";
        let base_ok = scope.base == "state_root" || scope.base == "temporary_directory";
        let store_ok = scope.kind != "workload_workspace"
            || scope
                .workload_store_relative_path
                .as_deref()
                .is_some_and(|path| safe_relative(path));
        if !kind_ok || !base_ok || !safe_relative(&scope.relative_path) || !store_ok {
            return Err("build_cache_retention_scope_invalid".to_string());
        }
    }
    Ok(())
}

fn safe_relative(value: &str) -> bool {
    !value.is_empty()
        && !Path::new(value).is_absolute()
        && !Path::new(value)
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
}

fn path_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| "build_cache_path_not_utf8".to_string())
}
