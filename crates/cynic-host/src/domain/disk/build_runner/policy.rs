use std::{fs, path::Path};

use serde::Deserialize;

use super::types::{
    BUILD_CACHE_PLAN_SCHEMA, BuildCacheDecision, BuildCacheDesiredState, BuildCacheObservation,
    BuildCachePathState, BuildCachePlan,
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

#[derive(Deserialize)]
struct BuildLifecycle {
    schema: String,
    minimum_available_bytes: u64,
    canonical_target_relative_path: String,
    canonical_cache_reclaim: CacheReclaim,
    noncanonical_cache_candidates: Vec<CacheCandidate>,
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
    Ok(BuildCacheDesiredState {
        node_id: node_id.to_string(),
        state_root: path_string(state_root)?,
        canonical_target: path_string(&canonical_target)?,
        minimum_available_bytes: policy.minimum_available_bytes,
        reclaim_profile: policy.canonical_cache_reclaim.target_profile,
        preserved_profiles: policy.canonical_cache_reclaim.preserve_profiles,
        noncanonical_cache_paths: noncanonical_cache_paths
            .iter()
            .map(|path| path_string(path))
            .collect::<Result<Vec<_>, _>>()?,
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
    BuildCachePlan {
        schema: BUILD_CACHE_PLAN_SCHEMA,
        status: match decision {
            BuildCacheDecision::Satisfied => "satisfied",
            BuildCacheDecision::ReclaimCanonicalProfile => "planned",
            BuildCacheDecision::Blocked => "blocked",
        },
        node_id: desired.node_id.clone(),
        mutates: decision == BuildCacheDecision::ReclaimCanonicalProfile,
        decision,
        target_profile: desired.reclaim_profile.clone(),
        target_path: observed.reclaim_profile_path.clone(),
        preserved_profiles: desired.preserved_profiles.clone(),
        minimum_available_bytes: desired.minimum_available_bytes,
        observed_available_bytes: observed.available_bytes,
        blockers,
        rollback_intent: "none_reproducible_build_cache_only",
    }
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
