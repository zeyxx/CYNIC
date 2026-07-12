use chrono::Utc;
use serde::Serialize;

use crate::build_attestation::RuntimeArtifactDeclarationDrift;
use crate::network_actuator_status::{
    NetworkActuatorActivationStatus, activation_status as network_actuator_activation_status,
};
use crate::network_repair::{NetworkRepairPlan, build_dry_run_plan};
use crate::network_root_policy::{NetworkRootPolicy, build_root_policy};
use crate::observe::NodeObservation;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkEvidence {
    schema: &'static str,
    generated_at: String,
    observation: serde_json::Value,
    repair_plan: NetworkRepairPlan,
    root_policy: NetworkRootPolicy,
    actuator_activation: NetworkActuatorActivationStatus,
    supply_chain_drift: RuntimeArtifactDeclarationDrift,
    devsecops_gaps: Vec<NetworkGap>,
}

#[derive(Debug, Clone, Serialize)]
struct NetworkGap {
    id: &'static str,
    severity: &'static str,
    layer: &'static str,
    status: &'static str,
    risk: &'static str,
    next_control: &'static str,
}

pub(crate) fn build_evidence(observation: &NodeObservation) -> NetworkEvidence {
    build_evidence_with_runtime(observation, network_actuator_activation_status(observation))
}

fn build_evidence_with_runtime(
    observation: &NodeObservation,
    actuator_activation: NetworkActuatorActivationStatus,
) -> NetworkEvidence {
    NetworkEvidence {
        schema: "cynic.node_network_evidence.v1",
        generated_at: Utc::now().to_rfc3339(),
        observation: serde_json::to_value(observation).unwrap_or_else(
            |error| serde_json::json!({ "serialization_error": error.to_string() }),
        ),
        repair_plan: build_dry_run_plan(observation),
        root_policy: build_root_policy(observation),
        actuator_activation,
        supply_chain_drift: observation.runtime_artifact_declaration_drift(),
        devsecops_gaps: devsecops_gaps(),
    }
}

fn devsecops_gaps() -> Vec<NetworkGap> {
    vec![
        NetworkGap {
            id: "network_effectuator_missing",
            severity: "critical",
            layer: "runtime_actuation",
            status: "partially_closed_by_bundle_surface",
            risk: "network-actuator helper, policy, and sudoers bundle now exist, but they are not yet installed and proved on runtime nodes",
            next_control: "install the network-actuator bundle on target nodes and require before/after cynic-node evidence for every apply",
        },
        NetworkGap {
            id: "deprecated_network_owner_in_desired_state",
            severity: "critical",
            layer: "desired_state",
            status: "open",
            risk: "registry and MCP surfaces may still route network authority to a deprecated organ",
            next_control: "move network_policy owner and MCP implementations to cynic-node diagnostics plus network-actuator execution",
        },
        NetworkGap {
            id: "root_network_policy_missing",
            severity: "high",
            layer: "privilege_boundary",
            status: "partially_closed_by_bundle_surface",
            risk: "network remediation now has a structured least-privilege policy plus installable helper/sudoers bundle, but denial proof and runtime installation are not yet enforced on target nodes",
            next_control: "install the generated helper and sudoers on target nodes, then prove raw ip/wg access remains denied outside the helper",
        },
        NetworkGap {
            id: "post_repair_evidence_not_enforced",
            severity: "high",
            layer: "evidence",
            status: "partially_closed_by_this_command",
            risk: "repairs can be planned, but success/failure must be measured consistently after mutation",
            next_control: "use cynic-node network evidence before and after every network-actuator apply",
        },
        NetworkGap {
            id: "node_binary_supply_chain_unproven",
            severity: "high",
            layer: "supply_chain",
            status: "partially_closed_by_runtime_attestation",
            risk: "runtime now attests the launched binary, but forge-built provenance and declared installation path are not yet enforced",
            next_control: "bind runtime attestation to forge-built artifact hash and declared installation path",
        },
        NetworkGap {
            id: "fallback_provider_plane_still_required",
            severity: "medium",
            layer: "network_resilience",
            status: "open",
            risk: "provider fallback is currently the live resurrection path; canonical Synapse is not self-sustaining yet",
            next_control: "make cynic-node observe report mesh_active without depending on provider fallback routing",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{build_evidence_with_runtime, *};
    use crate::domain::node_observation::NodeRuntimeCapabilities;
    use crate::network_actuator_status::types::{
        NetworkActuatorChecks, NetworkActuatorProofRequirement, RootAuthorityCheck,
    };

    #[test]
    fn evidence_contains_observation_repair_plan_and_gaps() {
        let observation = NodeObservation::synthetic_for_tests(
            NodeRuntimeCapabilities {
                provider_fallback_active: true,
                synapse_route_active: false,
                wireguard_active: false,
                critical_dns_ok: true,
                critical_dns_canonical: true,
                critical_tcp_open: true,
            },
            Some("192.168.0.37".to_string()),
        );

        let evidence = build_evidence_with_runtime(
            &observation,
            NetworkActuatorActivationStatus {
                schema: "cynic.network_actuator.activation_status.v1",
                generated_at: "1970-01-01T00:00:00+00:00".to_string(),
                status: "missing_activation",
                host_profile: "test-host",
                helper_path: "/missing/helper".to_string(),
                policy_path: "/missing/policy".to_string(),
                sudoers_target: "/missing/sudoers".to_string(),
                sudoers_rule: "test-rule".to_string(),
                bundle_dir: "/missing/bundle".to_string(),
                bundle_complete: false,
                bundle_evidence_source: "default_bundle_dir",
                next_actions: Vec::new(),
                proof_requirements: Vec::<NetworkActuatorProofRequirement>::new(),
                root_authority: RootAuthorityCheck {
                    command: "sudo -n true".to_string(),
                    sudo_noninteractive_available: false,
                    exit_code: Some(1),
                    stderr: "password required".to_string(),
                },
                checks: NetworkActuatorChecks {
                    helper_exists: false,
                    helper_mode_0755: false,
                    helper_executable: false,
                    policy_exists: false,
                    policy_mode_0644: false,
                    sudoers_exists: false,
                    sudoers_mode_0440: false,
                    sudoers_rule_matches: false,
                    sudoers_no_raw_ip: true,
                    sudoers_no_raw_wg: true,
                    sudoers_no_shell: true,
                },
                host_resolution: None,
                host_substrate: serde_json::json!({}),
                host_substrate_plan: serde_json::json!({}),
                host_bootstrap: serde_json::json!({}),
                host_bootstrap_plan: serde_json::json!({}),
                missing: Vec::new(),
            },
        );

        assert_eq!(evidence.schema, "cynic.node_network_evidence.v1");
        assert!(!evidence.devsecops_gaps.is_empty());
        assert_eq!(evidence.root_policy.schema, "cynic.network_root_policy.v1");
        assert_eq!(
            evidence.actuator_activation.schema,
            "cynic.network_actuator.activation_status.v1"
        );
        assert_eq!(
            evidence.supply_chain_drift.drift_status,
            "artifact_missing_from_registry"
        );
        assert_eq!(
            evidence.observation["status"],
            serde_json::json!("provider_fallback_active")
        );
        assert_eq!(
            evidence.observation["network_mesh_status"]["runtime_status"],
            serde_json::json!("provider_fallback_active")
        );
        assert_eq!(
            evidence.observation["network_mesh_status"]["dns_status"],
            serde_json::json!("ok")
        );
        assert_eq!(
            evidence.observation["runtime_observation"]["schema"],
            serde_json::json!("cynic.network_mesh.runtime_observation.v1")
        );
    }
}
