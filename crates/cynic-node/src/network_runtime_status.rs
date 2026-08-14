use chrono::Utc;
use serde::Serialize;
use serde_json::{Value, json};

use crate::dns_activation_status;
use crate::domain::node_observation::NodeRuntimeSeverity;
use crate::domain::node_observation::NodeRuntimeStatus;
use crate::network_actuator_status::{NetworkActuatorActivationStatus, activation_status};
use crate::network_host_substrate;
use crate::observe::{NodeObservation, observe_payload};
use crate::provider_fallback_status::build_provider_fallback_status;
use cynic_host::registry::HostProfileResolution;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkRuntimeStatus {
    schema: &'static str,
    generated_at: String,
    pub(crate) status: &'static str,
    pub(crate) severity: &'static str,
    hostname: String,
    host_profile: &'static str,
    registry: String,
    observation: Value,
    pub(crate) actuator_activation: NetworkActuatorActivationStatus,
    blockers: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host_resolution: Option<HostProfileResolution>,
    host_substrate: Value,
    host_substrate_plan: Value,
    host_bootstrap: Value,
    host_bootstrap_plan: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    dns_activation: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_fallback_status: Option<Value>,
}

pub(crate) fn build_network_runtime_status(
    registry: &str,
    resolve_dns: bool,
    check_provider_connectivity: bool,
    provider_hosts_path: &str,
    hosts_path: &str,
    corefile_path: &str,
    service_path: &str,
    resolved_dropin_path: &str,
) -> Result<NetworkRuntimeStatus, Box<dyn std::error::Error>> {
    let observation = observe_payload(Some(registry));
    let host_context = network_host_substrate::observe_network_host_runtime_context();
    let actuator_activation = activation_status(&observation);
    let dns_activation = if resolve_dns {
        Some(serde_json::to_value(
            dns_activation_status::build_dns_activation_status(
                registry,
                hosts_path,
                corefile_path,
                service_path,
                resolved_dropin_path,
                true,
            )?,
        )?)
    } else {
        None
    };

    let provider_fallback_status = if check_provider_connectivity {
        Some(serde_json::to_value(build_provider_fallback_status(
            registry,
            provider_hosts_path,
            true,
        )?)?)
    } else {
        None
    };
    build_network_runtime_status_with_runtime(
        registry,
        observation,
        host_context.host_profile,
        host_context.host_resolution.clone(),
        serde_json::to_value(&host_context.host_substrate)?,
        serde_json::to_value(&host_context.host_substrate_plan)?,
        serde_json::to_value(&host_context.host_bootstrap)?,
        serde_json::to_value(&host_context.host_bootstrap_plan)?,
        network_host_substrate::blocked_domains(&host_context),
        network_host_substrate::bootstrap_blocked(&host_context),
        actuator_activation,
        dns_activation,
        provider_fallback_status,
    )
}

fn build_network_runtime_status_with_runtime(
    registry: &str,
    observation: NodeObservation,
    host_profile: &'static str,
    host_resolution: Option<HostProfileResolution>,
    host_substrate: Value,
    host_substrate_plan: Value,
    host_bootstrap: Value,
    host_bootstrap_plan: Value,
    blocked_domains: Vec<&'static str>,
    bootstrap_blocked: bool,
    actuator_activation: NetworkActuatorActivationStatus,
    dns_activation: Option<Value>,
    provider_fallback_status: Option<Value>,
) -> Result<NetworkRuntimeStatus, Box<dyn std::error::Error>> {
    let status = match observation.status {
        NodeRuntimeStatus::Blocked => "blocked",
        NodeRuntimeStatus::ProviderFallbackActive => "provider_fallback_active",
        NodeRuntimeStatus::MeshActive => "mesh_active",
    };
    let severity = match observation.severity {
        NodeRuntimeSeverity::Clear => "clear",
        NodeRuntimeSeverity::Warning => "warning",
        NodeRuntimeSeverity::Critical => "critical",
    };

    let blockers = observation
        .blockers
        .iter()
        .map(|blocker| {
            json!({
                "kind": "runtime_blocker",
                "runtime_status": status,
                "blocker": blocker,
            })
        })
        .collect::<Vec<_>>();

    let mut blockers = blockers;
    for blocker in blocked_domains {
        blockers.push(json!({
            "kind": "host_substrate_blocker",
            "runtime_status": status,
            "blocker": blocker,
        }));
    }
    if bootstrap_blocked {
        blockers.push(json!({
            "kind": "host_bootstrap_blocker",
            "runtime_status": status,
            "blocker": "host_bootstrap_blocked",
        }));
    }

    Ok(NetworkRuntimeStatus {
        schema: "cynic.node.network_runtime_status.v1",
        generated_at: Utc::now().to_rfc3339(),
        status,
        severity,
        hostname: observation.hostname.clone(),
        host_profile,
        registry: registry.to_string(),
        observation: serde_json::to_value(&observation).unwrap_or_else(|error| {
            json!({
                "schema": "cynic.node_observation.v1",
                "serialization_error": error.to_string(),
            })
        }),
        actuator_activation,
        blockers,
        host_resolution,
        host_substrate,
        host_substrate_plan,
        host_bootstrap,
        host_bootstrap_plan,
        dns_activation,
        provider_fallback_status,
    })
}

#[cfg(test)]
mod tests {
    use super::build_network_runtime_status_with_runtime;
    use crate::domain::node_observation::NodeRuntimeCapabilities;
    use crate::network_actuator_status::NetworkActuatorActivationStatus;
    use crate::network_actuator_status::types::{
        NetworkActuatorChecks, NetworkActuatorProofRequirement, RootAuthorityCheck,
    };
    use crate::observe::NodeObservation;

    #[test]
    fn runtime_status_builder_is_pure_for_tests() {
        let observation = NodeObservation::synthetic_for_tests(
            NodeRuntimeCapabilities {
                provider_fallback_active: true,
                synapse_route_active: false,
                wireguard_active: false,
                critical_dns_ok: true,
                critical_dns_canonical: false,
                critical_tcp_open: true,
            },
            Some("192.168.0.37".to_string()),
        );

        let report = build_network_runtime_status_with_runtime(
            "test-registry.json",
            observation,
            "test-host",
            None,
            serde_json::json!({}),
            serde_json::json!({}),
            serde_json::json!({}),
            serde_json::json!({}),
            vec!["host_package_substrate_blocked"],
            true,
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
                    exit_code: None,
                    stderr: "test".to_string(),
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
            None,
            None,
        )
        .expect("runtime status");

        assert_eq!(report.schema, "cynic.node.network_runtime_status.v1");
        assert_eq!(report.status, "provider_fallback_active");
        assert_eq!(report.severity, "warning");
    }
}
