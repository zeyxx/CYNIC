use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const RUNTIME_PLACEMENT_DESIRED_STATE_SCHEMA: &str = "cynic.runtime_placement.desired_state.v1";
pub const RUNTIME_PLACEMENT_INTEGRITY_SCHEMA: &str = "cynic.runtime_placement.integrity.v1";
pub const RUNTIME_REGISTRY_SCHEMA: &str = "cynic.registry.v1.minimal";
pub const RUNTIME_PLACEMENT_PROMOTION_PLAN_SCHEMA: &str =
    "cynic.runtime_placement.promotion_plan.v1";
pub const KERNEL_IDENTITY_READ_BACK_SCHEMA: &str = "cynic.kernel.identity.read_back.v1";
pub const FLEET_DISCOVERY_REPORT_SCHEMA: &str = "cynic.fleet_discovery.v1";
pub const FLEET_INVENTORY_MAX_AGE_MS: u64 = 5 * 60 * 1000;
pub const FLEET_INVENTORY_FUTURE_TOLERANCE_MS: u64 = 5 * 1000;

pub fn fleet_inventory_read_back_violations(
    registry: &Value,
    observed_registry_digest: Option<&str>,
    observed_at_unix_ms: u64,
    now_unix_ms: u64,
) -> Vec<String> {
    let mut violations = Vec::new();
    let expected_digest = crate::digest_json(registry).ok();
    if observed_registry_digest != expected_digest.as_deref() {
        violations.push("fleet_inventory_registry_digest_mismatch".to_string());
    }
    if observed_at_unix_ms == 0 {
        violations.push("fleet_inventory_timestamp_missing".to_string());
    } else if observed_at_unix_ms > now_unix_ms.saturating_add(FLEET_INVENTORY_FUTURE_TOLERANCE_MS)
    {
        violations.push("fleet_inventory_timestamp_in_future".to_string());
    } else if now_unix_ms.saturating_sub(observed_at_unix_ms) > FLEET_INVENTORY_MAX_AGE_MS {
        violations.push("fleet_inventory_read_back_stale".to_string());
    }
    violations.sort();
    violations.dedup();
    violations
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetDiscoveryNode {
    pub node_id: String,
    pub provider: String,
    /// Provider-owned parent/host identity, when the substrate exposes one.
    pub provider_node: Option<String>,
    pub resource_id: Option<String>,
    pub resource_kind: Option<String>,
    pub power: String,
    pub address: Option<String>,
    pub declared_role: Option<String>,
    pub provider_observed: bool,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelPlacementCandidate {
    pub node_id: String,
    pub provider: String,
    pub host_node_id: Option<String>,
    pub resource_id: Option<String>,
    pub status: String,
    pub reasons: Vec<String>,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetKernelDiscoveryReport {
    pub schema: String,
    pub status: String,
    pub registry_digest: Option<String>,
    pub observed_at_unix_ms: Option<u64>,
    pub candidates: Vec<KernelPlacementCandidate>,
    pub blockers: Vec<String>,
}

pub fn derive_kernel_candidates(
    registry: &Value,
    observations: &[FleetDiscoveryNode],
) -> FleetKernelDiscoveryReport {
    let registry_digest = crate::digest_json(registry).ok();
    let mut candidates = observations.iter().map(candidate_for).collect::<Vec<_>>();

    if let Some(candidate) = declared_kernel_candidate(registry, &candidates) {
        candidates.push(candidate);
    }
    candidates.sort_by(|left, right| {
        left.node_id
            .cmp(&right.node_id)
            .then_with(|| {
                let left_rank = if left.status == "admissible" { 0 } else { 1 };
                let right_rank = if right.status == "admissible" { 0 } else { 1 };
                left_rank.cmp(&right_rank)
            })
            .then_with(|| left.provider.cmp(&right.provider))
    });
    candidates.dedup_by(|left, right| left.node_id == right.node_id);

    let mut blockers = Vec::new();
    if registry.get("schema").and_then(Value::as_str) != Some(crate::RUNTIME_REGISTRY_SCHEMA) {
        blockers.push("runtime_registry_schema_mismatch".to_string());
    }
    if registry_digest.is_none() {
        blockers.push("runtime_registry_digest_unavailable".to_string());
    }
    if !candidates
        .iter()
        .any(|candidate| candidate.status == "admissible")
    {
        blockers.push("kernel_candidate_not_observed".to_string());
    }
    if candidates
        .iter()
        .any(|candidate| candidate.status == "unknown")
    {
        blockers.push("kernel_candidate_observation_incomplete".to_string());
    }
    let status = if !blockers.is_empty() {
        "blocked"
    } else {
        "candidates_available"
    };

    FleetKernelDiscoveryReport {
        schema: FLEET_DISCOVERY_REPORT_SCHEMA.to_string(),
        status: status.to_string(),
        registry_digest,
        observed_at_unix_ms: None,
        candidates,
        blockers,
    }
}

fn candidate_for(observation: &FleetDiscoveryNode) -> KernelPlacementCandidate {
    let mut reasons = vec!["non_cortex_candidate_surface".to_string()];
    let mut blockers = observation.unknowns.clone();
    let reserved_role = observation.declared_role.as_deref().is_some_and(|role| {
        matches!(
            role,
            "cortex_host" | "hypervisor" | "storage_primary" | "forge_primary" | "build_runner"
        )
    });

    if reserved_role {
        return KernelPlacementCandidate {
            node_id: observation.node_id.clone(),
            provider: observation.provider.clone(),
            host_node_id: observation.provider_node.clone(),
            resource_id: observation.resource_id.clone(),
            status: "rejected".to_string(),
            reasons: vec!["role_reserved_for_existing_authority".to_string()],
            blockers: vec!["kernel_role_colocation_forbidden".to_string()],
            evidence_refs: observation.evidence_refs.clone(),
        };
    }
    if !observation.provider_observed {
        blockers.push("provider_inventory_unobserved".to_string());
    }
    if observation.resource_id.is_none() {
        blockers.push("resource_id_missing".to_string());
    }
    if observation.address.is_none() {
        blockers.push("network_address_unobserved".to_string());
    }
    if observation
        .resource_kind
        .as_deref()
        .is_none_or(str::is_empty)
    {
        blockers.push("unsupported_or_unknown_resource_kind".to_string());
    }
    if !matches!(
        observation.power.as_str(),
        "running" | "stopped" | "ready" | "Running" | "up"
    ) {
        blockers.push("power_state_unobserved".to_string());
    } else {
        reasons.push("provider_resource_identity_observed".to_string());
    }

    blockers.sort();
    blockers.dedup();
    let status = if blockers.is_empty() {
        "admissible"
    } else {
        "unknown"
    };
    KernelPlacementCandidate {
        node_id: observation.node_id.clone(),
        provider: observation.provider.clone(),
        host_node_id: observation.provider_node.clone(),
        resource_id: observation.resource_id.clone(),
        status: status.to_string(),
        reasons,
        blockers,
        evidence_refs: observation.evidence_refs.clone(),
    }
}

fn declared_kernel_candidate(
    registry: &Value,
    candidates: &[KernelPlacementCandidate],
) -> Option<KernelPlacementCandidate> {
    let kernel_placement = registry
        .get("runtime_placement")
        .and_then(Value::as_object)?
        .get("kernel")
        .and_then(Value::as_object)?;
    let node_id = kernel_placement.get("node_id")?.as_str()?.trim();
    if node_id.is_empty() {
        return None;
    }
    if candidates
        .iter()
        .any(|candidate| candidate.node_id == node_id)
    {
        return None;
    }

    let declared_role = kernel_placement
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or("");

    let nodes: &[Value] = registry
        .get("nodes")
        .and_then(Value::as_array)
        .map(|nodes| nodes.as_slice())
        .unwrap_or(&[]);
    let declared_node = nodes.iter().find(|node| {
        node.get("id").and_then(Value::as_str) == Some(node_id)
            || node
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|value| value == node_id)
            || node
                .get("proxmox_id")
                .and_then(Value::as_str)
                .is_some_and(|value| value == node_id)
    });

    let mut blockers = vec![
        "provider_inventory_unobserved".to_string(),
        "kernel_candidate_declared_not_observed".to_string(),
    ];
    let mut reasons = vec!["runtime_placement_declared".to_string()];
    let provider = declared_node
        .and_then(|node| node.get("provider_controls"))
        .and_then(|value| value.as_array())
        .and_then(|providers| providers.first())
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or("runtime_placement")
        .to_string();
    let provider_node = declared_node
        .and_then(|node| node.get("transports"))
        .and_then(|value| value.as_array())
        .and_then(|transports| transports.first())
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    let resource_id = declared_node
        .and_then(|node| node.get("proxmox_id").and_then(Value::as_str))
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    if declared_role.is_empty() {
        reasons.push("runtime_placement_role_unspecified".to_string());
    }

    blockers.sort();
    blockers.dedup();
    Some(KernelPlacementCandidate {
        node_id: node_id.to_string(),
        provider,
        host_node_id: provider_node,
        resource_id,
        status: "blocked".to_string(),
        reasons,
        blockers,
        evidence_refs: Vec::new(),
    })
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePlacementDesiredState {
    pub schema: String,
    pub authority: String,
    pub kernel: KernelPlacement,
    pub build_runner: BuildRunnerPlacement,
    pub integrity: RuntimePlacementIntegrity,
    pub invariants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelPlacement {
    pub status: String,
    pub node_id: Option<String>,
    pub host_node_id: Option<String>,
    pub role: String,
    pub service_name: String,
    pub endpoint_ref: Option<String>,
    pub transport_policy: String,
    #[serde(default)]
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerPlacement {
    pub node_id: String,
    pub role: String,
    pub transport_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePlacementIntegrity {
    pub schema: String,
    pub source_of_truth: String,
    pub binding: String,
    pub required_before_promotion: Vec<String>,
    pub unknown_policy: String,
    pub conflict_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePlacementPromotionPlan {
    pub schema: String,
    pub status: String,
    pub registry_digest: Option<String>,
    pub candidate_node_id: Option<String>,
    pub provider: Option<String>,
    pub host_node_id: Option<String>,
    pub resource_id: Option<String>,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
}

/// Validates a persisted, secret-free kernel identity checkpoint against the
/// declared placement. The provider resource binding is intentionally not
/// inferred here; that belongs to provider inventory read-back.
pub fn validate_kernel_identity_read_back(
    checkpoint: &Value,
    placement: &RuntimePlacementDesiredState,
    now_unix_ms: u64,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if checkpoint.get("schema").and_then(Value::as_str) != Some(KERNEL_IDENTITY_READ_BACK_SCHEMA) {
        blockers.push("kernel_identity_read_back_schema_mismatch".to_string());
    }
    if checkpoint.get("status").and_then(Value::as_str) != Some("observed") {
        blockers.push("kernel_identity_read_back_status_invalid".to_string());
    }
    let observed_at = checkpoint
        .get("observed_at_unix_ms")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    if observed_at == 0 {
        blockers.push("kernel_identity_read_back_timestamp_missing".to_string());
    } else if observed_at > now_unix_ms.saturating_add(crate::FLEET_INVENTORY_FUTURE_TOLERANCE_MS) {
        blockers.push("kernel_identity_read_back_timestamp_in_future".to_string());
    } else if now_unix_ms.saturating_sub(observed_at) > crate::FLEET_INVENTORY_MAX_AGE_MS {
        blockers.push("kernel_identity_read_back_stale".to_string());
    }
    let identity = checkpoint.get("identity");
    let parsed = identity
        .cloned()
        .and_then(|value| serde_json::from_value::<crate::ServiceIdentityEvidence>(value).ok());
    let Some(identity) = parsed else {
        blockers.push("kernel_identity_read_back_identity_invalid".to_string());
        return normalized_blockers(blockers);
    };
    if identity.schema != "cynic.kernel.identity.v1" {
        blockers.push("kernel_identity_schema_mismatch".to_string());
    }
    if identity.service != placement.kernel.service_name {
        blockers.push("kernel_identity_service_mismatch".to_string());
    }
    if placement.kernel.node_id.as_deref() != Some(identity.node_id.as_str()) {
        blockers.push("kernel_identity_node_mismatch".to_string());
    }
    if identity.hostname != identity.node_id {
        blockers.push("kernel_identity_hostname_mismatch".to_string());
    }
    if identity.build.git_sha.trim().is_empty() || identity.build.version.trim().is_empty() {
        blockers.push("kernel_identity_build_proof_missing".to_string());
    }
    let transport_url = checkpoint
        .get("transport")
        .and_then(|value| value.get("url"))
        .and_then(Value::as_str);
    if transport_url != placement.kernel.endpoint_ref.as_deref() {
        blockers.push("kernel_identity_endpoint_mismatch".to_string());
    }
    if checkpoint
        .get("proof")
        .and_then(|value| value.get("identity_matches_declared_node"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        blockers.push("kernel_identity_declaration_match_unproved".to_string());
    }
    normalized_blockers(blockers)
}

fn normalized_blockers(mut blockers: Vec<String>) -> Vec<String> {
    blockers.sort();
    blockers.dedup();
    blockers
}

/// Reconciles observed provider candidates with the registry without mutating
/// either one. A plan is promotable only when exactly one candidate has a
/// matching content-addressed registry snapshot and a provider-observed host.
pub fn plan_runtime_placement_promotion(
    registry: &Value,
    report: &crate::FleetKernelDiscoveryReport,
    now_unix_ms: u64,
) -> RuntimePlacementPromotionPlan {
    let registry_digest = crate::digest_json(registry).ok();
    let mut blockers = Vec::new();
    if report.schema != crate::FLEET_DISCOVERY_REPORT_SCHEMA {
        blockers.push("kernel_discovery_report_schema_mismatch".to_string());
    }
    if report.registry_digest != registry_digest {
        blockers.push("kernel_discovery_registry_digest_mismatch".to_string());
    }
    match report.observed_at_unix_ms {
        Some(observed_at) => blockers.extend(crate::fleet_inventory_read_back_violations(
            registry,
            report.registry_digest.as_deref(),
            observed_at,
            now_unix_ms,
        )),
        None => blockers.push("kernel_discovery_timestamp_missing".to_string()),
    }
    if report.status != "candidates_available" {
        blockers.push("kernel_discovery_not_promotable".to_string());
    }
    let admissible = report
        .candidates
        .iter()
        .filter(|candidate| candidate.status == "admissible")
        .collect::<Vec<_>>();
    if admissible.len() != 1 {
        blockers.push("kernel_candidate_count_not_unique".to_string());
    }
    let candidate = admissible.first().copied();
    if candidate.is_some_and(|candidate| candidate.host_node_id.is_none()) {
        blockers.push("kernel_candidate_host_unobserved".to_string());
    }
    if candidate.is_some_and(|candidate| {
        !candidate
            .evidence_refs
            .iter()
            .any(|reference| reference == "provider_inventory_read_back")
    }) {
        blockers.push("kernel_candidate_provider_read_back_missing".to_string());
    }
    if let Some(candidate) = candidate {
        if candidate.node_id == "cynic-host-cortex"
            || candidate.host_node_id.as_deref() == Some("cynic-host-cortex")
        {
            blockers.push("kernel_candidate_cortex_colocation_forbidden".to_string());
        }
    }
    blockers.extend(report.blockers.iter().cloned());
    blockers = normalized_blockers(blockers);
    RuntimePlacementPromotionPlan {
        schema: RUNTIME_PLACEMENT_PROMOTION_PLAN_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            "ready"
        } else {
            "blocked"
        }
        .to_string(),
        registry_digest,
        candidate_node_id: candidate.map(|candidate| candidate.node_id.clone()),
        provider: candidate.map(|candidate| candidate.provider.clone()),
        host_node_id: candidate.and_then(|candidate| candidate.host_node_id.clone()),
        resource_id: candidate.and_then(|candidate| candidate.resource_id.clone()),
        evidence_refs: candidate
            .map(|candidate| candidate.evidence_refs.clone())
            .unwrap_or_default(),
        blockers,
    }
}

/// Pure rule: validates whether a candidate node's load tier permits promotion.
/// A node in `Survival` state (saturated CPU/RAM or pressure) cannot be promoted
/// to an active runtime role (such as `kernel_primary` or `build_runner`).
pub fn validate_candidate_admissibility(
    candidate_node_id: &str,
    load_tier: crate::capacity::LoadTier,
) -> Result<(), String> {
    if load_tier == crate::capacity::LoadTier::Saturated {
        Err(format!(
            "candidate_{}_host_saturated",
            candidate_node_id
        ))
    } else {
        Ok(())
    }
}

/// Reconciles observed provider candidates and their load tier with the registry.
/// Rejects candidates currently in `LoadTier::Saturated`.
pub fn plan_runtime_placement_promotion_with_capacity(
    registry: &Value,
    report: &crate::FleetKernelDiscoveryReport,
    now_unix_ms: u64,
    candidate_load_tiers: &[(&str, crate::capacity::LoadTier)],
) -> RuntimePlacementPromotionPlan {
    let mut plan = plan_runtime_placement_promotion(registry, report, now_unix_ms);
    if let Some(candidate_id) = &plan.candidate_node_id {
        for (node_id, state) in candidate_load_tiers {
            if node_id == candidate_id {
                if let Err(blocker) = validate_candidate_admissibility(node_id, *state) {
                    plan.blockers.push(blocker);
                    plan.blockers = normalized_blockers(plan.blockers);
                    plan.status = "blocked".to_string();
                }
                break;
            }
        }
    }
    plan
}

pub fn validate_runtime_placement_promotion_plan(
    plan: &RuntimePlacementPromotionPlan,
    registry: &Value,
) -> Vec<String> {
    let mut violations = Vec::new();
    if plan.schema != RUNTIME_PLACEMENT_PROMOTION_PLAN_SCHEMA {
        violations.push("runtime_placement_promotion_schema_mismatch".to_string());
    }
    let expected_digest = crate::digest_json(registry).ok();
    if plan.registry_digest != expected_digest {
        violations.push("runtime_placement_promotion_registry_digest_mismatch".to_string());
    }
    if !matches!(plan.status.as_str(), "ready" | "blocked") {
        violations.push("runtime_placement_promotion_status_invalid".to_string());
    }
    if plan.status == "ready" {
        if plan.candidate_node_id.is_none() {
            violations.push("runtime_placement_promotion_candidate_missing".to_string());
        }
        if plan.provider.is_none() {
            violations.push("runtime_placement_promotion_provider_missing".to_string());
        }
        if plan.host_node_id.is_none() {
            violations.push("runtime_placement_promotion_host_missing".to_string());
        }
        if plan.resource_id.is_none() {
            violations.push("runtime_placement_promotion_resource_missing".to_string());
        }
        if !plan
            .evidence_refs
            .iter()
            .any(|reference| reference == "provider_inventory_read_back")
        {
            violations.push("runtime_placement_promotion_provider_read_back_missing".to_string());
        }
        if !plan.blockers.is_empty() {
            violations.push("runtime_placement_promotion_ready_with_blockers".to_string());
        }
    }
    violations.sort();
    violations.dedup();
    violations
}

pub fn validate_runtime_placement(
    placement: &RuntimePlacementDesiredState,
    registry: &Value,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if placement.schema != RUNTIME_PLACEMENT_DESIRED_STATE_SCHEMA {
        blockers.push("runtime_placement_schema_mismatch".to_string());
    }
    if placement.authority != "registry.nodes" {
        blockers.push("runtime_placement_authority_mismatch".to_string());
    }
    if placement.integrity.schema != RUNTIME_PLACEMENT_INTEGRITY_SCHEMA {
        blockers.push("runtime_placement_integrity_schema_mismatch".to_string());
    }
    if placement.integrity.source_of_truth != "registry.nodes" {
        blockers.push("runtime_placement_integrity_source_mismatch".to_string());
    }
    if registry.get("schema").and_then(Value::as_str) != Some(RUNTIME_REGISTRY_SCHEMA) {
        blockers.push("runtime_registry_schema_mismatch".to_string());
    }
    if placement.kernel.status != "declared" {
        blockers.push("kernel_desired_node_not_declared".to_string());
    }
    if placement.kernel.node_id.is_none() {
        blockers.push("kernel_desired_node_missing".to_string());
    }
    if placement.kernel.host_node_id.is_none() {
        blockers.push("kernel_desired_host_missing".to_string());
    }
    if placement.kernel.endpoint_ref.is_none() {
        blockers.push("kernel_desired_endpoint_missing".to_string());
    }
    if placement.kernel.service_name.trim().is_empty() {
        blockers.push("kernel_desired_service_missing".to_string());
    }
    let nodes = registry.get("nodes").and_then(Value::as_array);
    let Some(nodes) = nodes else {
        blockers.push("runtime_registry_nodes_missing".to_string());
        return blockers;
    };
    if let Some(node_id) = placement.kernel.node_id.as_deref() {
        let matching_nodes = nodes
            .iter()
            .filter(|node| {
                node.get("id").and_then(Value::as_str) == Some(node_id)
                    && node.get("role").and_then(Value::as_str) == Some("kernel_primary")
            })
            .collect::<Vec<_>>();
        if matching_nodes.len() != 1 {
            blockers.push("kernel_desired_node_not_unique".to_string());
        } else if !matching_nodes[0]
            .get("services")
            .and_then(Value::as_array)
            .is_some_and(|services| {
                services.iter().any(|service| {
                    service.get("name").and_then(Value::as_str)
                        == Some(placement.kernel.service_name.as_str())
                })
            })
        {
            blockers.push("kernel_desired_service_not_declared".to_string());
        }
    }
    if let Some(host_id) = placement.kernel.host_node_id.as_deref() {
        let matches = nodes
            .iter()
            .filter(|node| {
                node.get("id").and_then(Value::as_str) == Some(host_id)
                    && node.get("role").and_then(Value::as_str) != Some("cortex_host")
            })
            .count();
        if matches != 1 {
            blockers.push("kernel_desired_host_not_unique_or_is_cortex".to_string());
        }
    }
    if placement.build_runner.role != "build_runner" {
        blockers.push("build_runner_placement_role_mismatch".to_string());
    }
    blockers
}

#[cfg(test)]
mod tests {
    use super::{
        FleetDiscoveryNode, RuntimePlacementDesiredState, derive_kernel_candidates,
        plan_runtime_placement_promotion, validate_kernel_identity_read_back,
        validate_runtime_placement,
    };
    fn sample_registry_and_placement() -> (serde_json::Value, RuntimePlacementDesiredState) {
        let placement = RuntimePlacementDesiredState {
            schema: crate::RUNTIME_PLACEMENT_DESIRED_STATE_SCHEMA.to_string(),
            authority: "registry.nodes".to_string(),
            kernel: super::KernelPlacement {
                status: "declared".to_string(),
                node_id: Some("cynic-vm-v2".to_string()),
                host_node_id: Some("pve".to_string()),
                role: "kernel_primary".to_string(),
                service_name: "cynic-kernel".to_string(),
                endpoint_ref: Some("http://10.10.10.140:3030/identity".to_string()),
                transport_policy: "declared_service_endpoint".to_string(),
                blockers: vec![],
            },
            build_runner: super::BuildRunnerPlacement {
                node_id: "builder-a".to_string(),
                role: "build_runner".to_string(),
                transport_policy: "internal_bridge".to_string(),
            },
            integrity: super::RuntimePlacementIntegrity {
                schema: crate::RUNTIME_PLACEMENT_INTEGRITY_SCHEMA.to_string(),
                source_of_truth: "registry.nodes".to_string(),
                binding: "exact_node_match".to_string(),
                required_before_promotion: vec![],
                unknown_policy: "preserve".to_string(),
                conflict_policy: "reject".to_string(),
            },
            invariants: vec![],
        };
        let registry = serde_json::json!({
            "schema": crate::RUNTIME_REGISTRY_SCHEMA,
            "nodes": [
                {
                    "id": "cynic-vm-v2",
                    "role": "kernel_primary",
                    "services": [
                        { "name": "cynic-kernel" }
                    ]
                },
                {
                    "id": "pve",
                    "role": "hypervisor"
                },
                {
                    "id": "builder-a",
                    "role": "build_runner"
                }
            ]
        });
        (registry, placement)
    }

    #[test]
    fn observed_non_cortex_kernel_placement_is_validated_from_registry() {
        let (registry, placement) = sample_registry_and_placement();
        let blockers = validate_runtime_placement(&placement, &registry);
        assert!(
            blockers.is_empty(),
            "unexpected placement blockers: {blockers:?}"
        );
    }

    #[test]
    fn kernel_identity_checkpoint_binds_to_declared_placement() {
        let (_registry, placement) = sample_registry_and_placement();
        let checkpoint: serde_json::Value = serde_json::from_str(include_str!(
            "../../../infra/runtime-checkpoints/2026-08-04-kernel-vm-v2-identity.json"
        ))
        .expect("identity checkpoint");
        let blockers =
            validate_kernel_identity_read_back(&checkpoint, &placement, 1_785_851_214_000);
        assert!(
            blockers.is_empty(),
            "unexpected identity blockers: {blockers:?}"
        );
    }

    #[test]
    fn promotion_plan_requires_one_observed_non_cortex_candidate() {
        let registry = serde_json::json!({"schema": crate::RUNTIME_REGISTRY_SCHEMA});
        let report = derive_kernel_candidates(
            &registry,
            &[FleetDiscoveryNode {
                node_id: "qemu/120".into(),
                provider: "proxmox".into(),
                provider_node: Some("pve".into()),
                resource_id: Some("qemu/120".into()),
                resource_kind: Some("qemu".into()),
                power: "running".into(),
                address: Some("10.0.0.120".into()),
                declared_role: None,
                provider_observed: true,
                evidence_refs: vec!["provider_inventory_read_back".into()],
                unknowns: vec![],
            }],
        );
        let mut report = report;
        report.observed_at_unix_ms = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        );
        let plan = plan_runtime_placement_promotion(
            &registry,
            &report,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        );
        assert_eq!(plan.status, "ready");
        assert_eq!(plan.host_node_id.as_deref(), Some("pve"));
        assert!(crate::validate_runtime_placement_promotion_plan(&plan, &registry).is_empty());
    }

    #[test]
    fn promotion_plan_rejects_a_stale_registry_binding() {
        let registry = serde_json::json!({"schema": crate::RUNTIME_REGISTRY_SCHEMA});
        let mut report = crate::derive_kernel_candidates(&registry, &[]);
        report.registry_digest = Some("stale-digest".into());
        let plan = plan_runtime_placement_promotion(&registry, &report, 100);
        assert!(
            plan.blockers
                .contains(&"kernel_discovery_registry_digest_mismatch".to_string())
        );
        assert_eq!(plan.status, "blocked");
    }

    #[test]
    fn promotion_plan_blocks_declared_kernel_without_inventory_observation() {
        let registry = serde_json::json!({
            "schema": crate::RUNTIME_REGISTRY_SCHEMA,
            "runtime_placement": {
                "kernel": {
                    "node_id": "qemu/120",
                    "role": "kernel_primary",
                    "service_name": "cynic-kernel",
                    "transport_policy": "declared_service_endpoint",
                    "status": "declared"
                }
            },
            "nodes": [
                {
                    "id": "qemu/120",
                    "role": "kernel_primary",
                    "proxmox_id": "qemu/120"
                }
            ]
        });
        let mut report = derive_kernel_candidates(&registry, &[]);
        report.observed_at_unix_ms = Some(1);
        let now_unix_ms = 1;
        let plan = plan_runtime_placement_promotion(&registry, &report, now_unix_ms);
        assert_eq!(plan.status, "blocked");
        assert!(plan.candidate_node_id.is_none());
        assert!(
            plan.blockers
                .contains(&"kernel_discovery_not_promotable".to_string())
        );
        assert!(
            plan.blockers
                .contains(&"kernel_candidate_not_observed".to_string())
        );
        assert!(plan.provider.is_none());
    }

    #[test]
    fn saturated_host_blocks_candidate_promotion() {
        let registry = serde_json::json!({"schema": crate::RUNTIME_REGISTRY_SCHEMA});
        let report = derive_kernel_candidates(
            &registry,
            &[FleetDiscoveryNode {
                node_id: "qemu/120".into(),
                provider: "proxmox".into(),
                provider_node: Some("pve".into()),
                resource_id: Some("qemu/120".into()),
                resource_kind: Some("qemu".into()),
                power: "running".into(),
                address: Some("10.0.0.120".into()),
                declared_role: None,
                provider_observed: true,
                evidence_refs: vec!["provider_inventory_read_back".into()],
                unknowns: vec![],
            }],
        );
        let mut report = report;
        report.observed_at_unix_ms = Some(100);

        // HighPerformance or Standard allows promotion
        let plan = super::plan_runtime_placement_promotion_with_capacity(
            &registry,
            &report,
            100,
            &[("qemu/120", crate::capacity::LoadTier::Headroom)],
        );
        assert_eq!(plan.status, "ready");

        // Survival blocks promotion
        let blocked_plan = super::plan_runtime_placement_promotion_with_capacity(
            &registry,
            &report,
            100,
            &[("qemu/120", crate::capacity::LoadTier::Saturated)],
        );
        assert_eq!(blocked_plan.status, "blocked");
        assert!(
            blocked_plan
                .blockers
                .contains(&"candidate_qemu/120_host_saturated".to_string())
        );
    }
}
