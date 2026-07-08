use chrono::Utc;
use serde::Serialize;
use serde_json::{json, Value};

use crate::dns_activation_status;
use crate::domain::node_observation::NodeRuntimeStatus;
use crate::network_actuator_status::{activation_status, NetworkActuatorActivationStatus};
use crate::observe::observe_payload;
use crate::provider_fallback_status::build_provider_fallback_status;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkRuntimeStatus {
    schema: &'static str,
    generated_at: String,
    pub(crate) status: &'static str,
    pub(crate) severity: &'static str,
    hostname: String,
    registry: String,
    observation: Value,
    pub(crate) actuator_activation: NetworkActuatorActivationStatus,
    blockers: Vec<Value>,
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
    let status = match observation.status {
        NodeRuntimeStatus::Blocked => "blocked",
        NodeRuntimeStatus::ProviderFallbackActive => "provider_fallback_active",
        NodeRuntimeStatus::MeshActive => "mesh_active",
    };
    let severity = match observation.severity {
        crate::domain::node_observation::NodeRuntimeSeverity::Clear => "clear",
        crate::domain::node_observation::NodeRuntimeSeverity::Warning => "warning",
        crate::domain::node_observation::NodeRuntimeSeverity::Critical => "critical",
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
        .collect();

    let actuator_activation = activation_status(&observation);

    let dns_activation = if resolve_dns {
        Some(serde_json::to_value(
            dns_activation_status::build_dns_activation_status(
                registry,
                provider_hosts_path,
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
            registry, hosts_path, true,
        )?)?)
    } else {
        None
    };

    Ok(NetworkRuntimeStatus {
        schema: "cynic.node.network_runtime_status.v1",
        generated_at: Utc::now().to_rfc3339(),
        status,
        severity,
        hostname: observation.hostname.clone(),
        registry: registry.to_string(),
        observation: serde_json::to_value(&observation).unwrap_or_else(|error| {
            json!({
                "schema": "cynic.node_observation.v1",
                "serialization_error": error.to_string(),
            })
        }),
        actuator_activation,
        blockers,
        dns_activation,
        provider_fallback_status,
    })
}
