use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const FLEET_DISCOVERY_REPORT_SCHEMA: &str = "cynic.fleet.discovery.report.v1";
pub const FLEET_INVENTORY_REPORT_SCHEMA: &str = "cynic.fleet.inventory.report.v2";
pub const FLEET_KERNEL_PLACEMENT_BUNDLE_SCHEMA: &str = "cynic.fleet.kernel_placement_bundle.v1";
pub const FLEET_READ_BACK_BLOCKED_SCHEMA: &str = "cynic.fleet.read_back_blocked.v1";
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

pub fn validate_fleet_inventory_read_back(
    report: &Value,
    registry: &Value,
    now_unix_ms: u64,
) -> Vec<String> {
    let mut violations = Vec::new();
    if report.get("schema").and_then(Value::as_str) != Some(FLEET_INVENTORY_REPORT_SCHEMA) {
        violations.push("fleet_inventory_report_schema_mismatch".to_string());
    }
    if !matches!(
        report.get("status").and_then(Value::as_str),
        Some("ok" | "degraded" | "unavailable")
    ) {
        violations.push("fleet_inventory_report_status_invalid".to_string());
    }
    if !report.get("nodes").is_some_and(Value::is_array) {
        violations.push("fleet_inventory_nodes_missing".to_string());
    }
    if !report.get("summary").is_some_and(Value::is_object) {
        violations.push("fleet_inventory_summary_missing".to_string());
    }
    let digest = report.get("registry_digest").and_then(Value::as_str);
    let observed_at = report
        .get("observed_at_unix_ms")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    violations.extend(fleet_inventory_read_back_violations(
        registry,
        digest,
        observed_at,
        now_unix_ms,
    ));
    violations.sort();
    violations.dedup();
    violations
}

pub fn validate_kernel_discovery_read_back(
    report: &Value,
    registry: &Value,
    now_unix_ms: u64,
) -> Vec<String> {
    let mut violations = Vec::new();
    if report.get("schema").and_then(Value::as_str) != Some(FLEET_DISCOVERY_REPORT_SCHEMA) {
        violations.push("kernel_discovery_report_schema_mismatch".to_string());
    }
    if !matches!(
        report.get("status").and_then(Value::as_str),
        Some("blocked" | "candidates_available")
    ) {
        violations.push("kernel_discovery_report_status_invalid".to_string());
    }
    if !report.get("candidates").is_some_and(Value::is_array) {
        violations.push("kernel_discovery_candidates_missing".to_string());
    }
    if !report.get("blockers").is_some_and(Value::is_array) {
        violations.push("kernel_discovery_blockers_missing".to_string());
    }
    let digest = report.get("registry_digest").and_then(Value::as_str);
    let observed_at = report
        .get("observed_at_unix_ms")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    violations.extend(fleet_inventory_read_back_violations(
        registry,
        digest,
        observed_at,
        now_unix_ms,
    ));
    if let Some(candidates) = report.get("candidates").and_then(Value::as_array) {
        let mut node_ids = std::collections::BTreeSet::new();
        for candidate in candidates {
            let node_id = candidate.get("node_id").and_then(Value::as_str);
            if node_id.is_none() {
                violations.push("kernel_discovery_candidate_node_missing".to_string());
            } else if !node_ids.insert(node_id.unwrap_or_default()) {
                violations.push("kernel_discovery_candidate_duplicate_node".to_string());
            }
            if candidate.get("provider").and_then(Value::as_str).is_none() {
                violations.push("kernel_discovery_candidate_provider_missing".to_string());
            }
            if !candidate.get("evidence_refs").is_some_and(Value::is_array) {
                violations.push("kernel_discovery_candidate_evidence_missing".to_string());
            }
        }
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
        .and_then(Value::as_object)
        .and_then(|value| value.get("ssh"))
        .and_then(Value::as_object)
        .and_then(|value| value.get("target"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| {
            kernel_placement
                .get("host_node_id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        });
    let resource_id = declared_node
        .and_then(|node| node.get("proxmox_id").and_then(Value::as_str))
        .or_else(|| Some(node_id))
        .map(ToString::to_string);
    let resource_kind = resource_id.as_deref().and_then(resource_kind_from_id);

    let declared_role_blocked = matches!(
        declared_role,
        "cortex_host" | "hypervisor" | "storage_primary" | "forge_primary" | "build_runner"
    );
    if !declared_role_blocked {
        reasons.push("declared_role_candidate_surface".to_string());
    } else {
        blockers.push("kernel_role_colocation_forbidden".to_string());
        blockers.push("declared_role_reserved".to_string());
    }
    if provider_node.is_some() {
        reasons.push("kernel_declared_host_present".to_string());
    }
    if resource_id.is_some() {
        reasons.push("kernel_declared_resource_present".to_string());
    }
    if resource_kind.is_some() {
        reasons.push("runtime_placement_declared_resource_id".to_string());
    } else {
        blockers.push("runtime_placement_declared_resource_kind_unknown".to_string());
    }

    blockers.sort();
    blockers.dedup();
    Some(KernelPlacementCandidate {
        node_id: node_id.to_string(),
        provider,
        host_node_id: provider_node,
        resource_id,
        status: "unknown".to_string(),
        reasons,
        blockers,
        evidence_refs: vec!["runtime_placement_read_back".to_string()],
    })
}

fn resource_kind_from_id(resource_id: &str) -> Option<String> {
    resource_id
        .split_once('/')
        .filter(|(_, value)| !value.is_empty())
        .map(|value| value.0.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reserved_build_runner_is_not_a_kernel_candidate() {
        let report = derive_kernel_candidates(
            &json!({"schema": crate::RUNTIME_REGISTRY_SCHEMA}),
            &[FleetDiscoveryNode {
                node_id: "lxc/105".into(),
                provider: "proxmox".into(),
                provider_node: Some("pve".into()),
                resource_id: Some("lxc/105".into()),
                resource_kind: Some("lxc".into()),
                power: "running".into(),
                address: Some("10.10.10.119".into()),
                declared_role: Some("build_runner".into()),
                provider_observed: true,
                evidence_refs: vec!["provider_inventory_read_back".into()],
                unknowns: vec![],
            }],
        );
        assert_eq!(report.candidates[0].status, "rejected");
        assert!(
            report
                .blockers
                .contains(&"kernel_candidate_not_observed".to_string())
        );
    }

    #[test]
    fn unbound_running_vm_is_admissible_but_not_promoted() {
        let report = derive_kernel_candidates(
            &json!({"schema": crate::RUNTIME_REGISTRY_SCHEMA}),
            &[FleetDiscoveryNode {
                node_id: "qemu/120".into(),
                provider: "proxmox".into(),
                provider_node: Some("pve".into()),
                resource_id: Some("qemu/120".into()),
                resource_kind: Some("qemu".into()),
                power: "running".into(),
                address: Some("10.10.10.120".into()),
                declared_role: None,
                provider_observed: true,
                evidence_refs: vec!["provider_inventory_read_back".into()],
                unknowns: vec![],
            }],
        );
        assert_eq!(report.status, "candidates_available");
        assert_eq!(report.candidates[0].provider, "proxmox");
        assert_eq!(report.candidates[0].status, "admissible");
    }

    #[test]
    fn declared_kernel_node_is_reported_when_unobserved() {
        let report = derive_kernel_candidates(
            &json!({
                "schema": crate::RUNTIME_REGISTRY_SCHEMA,
                "runtime_placement": {
                    "kernel": {
                        "node_id": "qemu/120",
                        "host_node_id": "pve-1",
                        "role": "kernel_primary",
                        "service_name": "cynic-kernel",
                        "transport_policy": "declared_service_endpoint_then_read_back",
                        "status": "declared",
                        "endpoint_ref": "http://10.10.10.120:3030/identity"
                    }
                },
                "nodes": [
                    {
                        "id": "qemu/120",
                        "role": "kernel_primary",
                        "transports": {"ssh": {"target": "pve-1"}},
                        "proxmox_id": "qemu/120"
                    }
                ]
            }),
            &[],
        );
        assert_eq!(report.status, "blocked");
        assert_eq!(report.candidates.len(), 1);
        assert_eq!(report.candidates[0].node_id, "qemu/120");
        assert_eq!(report.candidates[0].status, "unknown");
        assert!(
            report.candidates[0]
                .evidence_refs
                .iter()
                .any(|value| value == "runtime_placement_read_back")
        );
        assert!(
            report
                .blockers
                .contains(&"kernel_candidate_not_observed".to_string())
        );
    }

    #[test]
    fn provider_agnostic_ready_node_keeps_its_host_identity() {
        let report = derive_kernel_candidates(
            &json!({"schema": crate::RUNTIME_REGISTRY_SCHEMA}),
            &[FleetDiscoveryNode {
                node_id: "k8s/node/worker-a".into(),
                provider: "kubernetes".into(),
                provider_node: Some("cluster-a".into()),
                resource_id: Some("k8s/node/worker-a".into()),
                resource_kind: Some("node".into()),
                power: "ready".into(),
                address: Some("10.0.0.20".into()),
                declared_role: None,
                provider_observed: true,
                evidence_refs: vec!["provider_inventory_read_back".into()],
                unknowns: vec![],
            }],
        );
        assert_eq!(report.status, "candidates_available");
        assert_eq!(
            report.candidates[0].host_node_id.as_deref(),
            Some("cluster-a")
        );
    }

    #[test]
    fn inventory_read_back_rejects_stale_or_unbound_observation() {
        let registry = json!({"schema": crate::RUNTIME_REGISTRY_SCHEMA});
        let digest = crate::digest_json(&registry).expect("registry digest");
        let violations = fleet_inventory_read_back_violations(
            &registry,
            Some(&digest),
            1,
            FLEET_INVENTORY_MAX_AGE_MS + 2,
        );
        assert!(violations.contains(&"fleet_inventory_read_back_stale".to_string()));

        let future = fleet_inventory_read_back_violations(&registry, Some("wrong"), 20_000, 1);
        assert!(future.contains(&"fleet_inventory_registry_digest_mismatch".to_string()));
        assert!(future.contains(&"fleet_inventory_timestamp_in_future".to_string()));
    }

    #[test]
    fn inventory_report_requires_the_machine_read_back_fields() {
        let registry = json!({"schema": crate::RUNTIME_REGISTRY_SCHEMA});
        let digest = crate::digest_json(&registry).expect("registry digest");
        let report = json!({
            "schema": FLEET_INVENTORY_REPORT_SCHEMA,
            "status": "degraded",
            "registry_digest": digest,
            "observed_at_unix_ms": 100,
            "nodes": [],
            "summary": {}
        });
        assert!(validate_fleet_inventory_read_back(&report, &registry, 100).is_empty());
        assert!(
            !validate_fleet_inventory_read_back(
                &json!({"schema": FLEET_INVENTORY_REPORT_SCHEMA}),
                &registry,
                100
            )
            .is_empty()
        );
    }

    #[test]
    fn kernel_discovery_report_requires_bound_fresh_candidates() {
        let registry = json!({"schema": crate::RUNTIME_REGISTRY_SCHEMA});
        let digest = crate::digest_json(&registry).expect("registry digest");
        let report = json!({
            "schema": FLEET_DISCOVERY_REPORT_SCHEMA,
            "status": "blocked",
            "registry_digest": digest,
            "observed_at_unix_ms": 100,
            "candidates": [],
            "blockers": ["kernel_candidate_not_observed"]
        });
        assert!(validate_kernel_discovery_read_back(&report, &registry, 100).is_empty());
    }
}
