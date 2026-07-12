use serde::Serialize;

use crate::domain::node_observation::{NodeRuntimeSeverity, NodeRuntimeStatus, NodeRuntimeVerdict};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WireguardTransportState {
    Active,
    RouteDrift,
    MeshAbsent,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WireguardTransportChecks {
    pub(crate) wg_show_has_output: bool,
    pub(crate) wg_quick_active: bool,
    pub(crate) synapse_route_present: bool,
    pub(crate) provider_fallback_still_required: bool,
    pub(crate) interface_present: bool,
    pub(crate) peer_present: bool,
    pub(crate) handshake_present: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WireguardTransportObservation {
    pub(crate) state: WireguardTransportState,
    pub(crate) checks: WireguardTransportChecks,
    pub(crate) drift_classes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DnsProjectionState {
    Active,
    ProjectionDrift,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsProjectionChecks {
    pub(crate) critical_hosts_resolve: bool,
    pub(crate) systemd_resolved_active: bool,
    pub(crate) provider_fallback_dependency: bool,
    pub(crate) resolved_matches_expected_synapse: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsProjectionObservation {
    pub(crate) state: DnsProjectionState,
    pub(crate) checks: DnsProjectionChecks,
    pub(crate) drift_classes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CanonicalNetworkMeshStatusProjection {
    pub(crate) runtime_status: String,
    pub(crate) dns_status: String,
    pub(crate) policy_status: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SharedRuntimeObservation {
    pub(crate) schema: &'static str,
    pub(crate) source: &'static str,
    pub(crate) status: String,
    pub(crate) severity: String,
    pub(crate) blockers: Vec<String>,
    pub(crate) missing: Vec<String>,
    pub(crate) drift: Vec<serde_json::Value>,
    pub(crate) blocking: Vec<serde_json::Value>,
    pub(crate) wireguard_transport: serde_json::Value,
    pub(crate) dns_projection: serde_json::Value,
}

pub(crate) fn wireguard_transport_observation(
    synapse_route_active: bool,
    provider_fallback_active: bool,
    wireguard_active: bool,
    wg_quick_active: bool,
    interface_present: bool,
    peer_present: bool,
    handshake_present: bool,
) -> WireguardTransportObservation {
    let checks = WireguardTransportChecks {
        wg_show_has_output: wireguard_active,
        wg_quick_active,
        synapse_route_present: synapse_route_active,
        provider_fallback_still_required: provider_fallback_active,
        interface_present,
        peer_present,
        handshake_present,
    };

    let (state, drift_classes) = if interface_present
        && peer_present
        && handshake_present
        && wg_quick_active
        && synapse_route_active
    {
        let drifts = if provider_fallback_active {
            vec!["provider_fallback_dependency"]
        } else {
            Vec::new()
        };
        (WireguardTransportState::Active, drifts)
    } else if synapse_route_active {
        (
            WireguardTransportState::MeshAbsent,
            vec!["mesh_transport_drift"],
        )
    } else {
        (
            WireguardTransportState::RouteDrift,
            vec!["mesh_transport_drift", "provider_fallback_dependency"],
        )
    };

    WireguardTransportObservation {
        state,
        checks,
        drift_classes,
    }
}

pub(crate) fn dns_projection_observation(
    dns_ok: bool,
    resolved_matches_expected_synapse: bool,
    provider_fallback_active: bool,
    systemd_resolved_active: bool,
) -> DnsProjectionObservation {
    let checks = DnsProjectionChecks {
        critical_hosts_resolve: dns_ok,
        systemd_resolved_active,
        provider_fallback_dependency: provider_fallback_active,
        resolved_matches_expected_synapse,
    };

    let mut drift_classes = Vec::new();
    if !dns_ok || !systemd_resolved_active || !resolved_matches_expected_synapse {
        drift_classes.push("dns_projection_drift");
    }
    if provider_fallback_active {
        drift_classes.push("provider_fallback_dependency");
    }

    DnsProjectionObservation {
        state: if drift_classes
            .iter()
            .any(|item| *item == "dns_projection_drift")
        {
            DnsProjectionState::ProjectionDrift
        } else {
            DnsProjectionState::Active
        },
        checks,
        drift_classes,
    }
}

pub(crate) fn canonical_network_mesh_status_projection(
    status: &NodeRuntimeStatus,
    critical_dns_ok: bool,
) -> CanonicalNetworkMeshStatusProjection {
    let runtime_status = match status {
        NodeRuntimeStatus::Blocked => "blocked",
        NodeRuntimeStatus::ProviderFallbackActive => "provider_fallback_active",
        NodeRuntimeStatus::MeshActive => "mesh_active",
    };
    CanonicalNetworkMeshStatusProjection {
        runtime_status: runtime_status.to_string(),
        dns_status: if critical_dns_ok {
            "ok".to_string()
        } else {
            "violation".to_string()
        },
        policy_status: "ok",
    }
}

pub(crate) fn shared_runtime_observation(
    verdict: &NodeRuntimeVerdict,
    wireguard_transport: &WireguardTransportObservation,
    dns_projection: &DnsProjectionObservation,
) -> SharedRuntimeObservation {
    let status = match verdict.status {
        NodeRuntimeStatus::Blocked => "blocked",
        NodeRuntimeStatus::ProviderFallbackActive => "provider_fallback_active",
        NodeRuntimeStatus::MeshActive => "mesh_active",
    }
    .to_string();
    let severity = match verdict.severity {
        NodeRuntimeSeverity::Clear => "clear",
        NodeRuntimeSeverity::Warning => "warning",
        NodeRuntimeSeverity::Critical => "critical",
    }
    .to_string();
    let blockers = verdict
        .blockers
        .iter()
        .map(|blocker| {
            serde_json::to_value(blocker).unwrap_or_else(|_| serde_json::json!("unknown"))
        })
        .filter_map(|value| value.as_str().map(|item| item.to_string()))
        .collect::<Vec<_>>();
    let drift = blockers
        .iter()
        .map(|blocker| {
            serde_json::json!({
                "kind": "runtime_blocker",
                "missing": blocker,
                "reason": "cynic-node observe reported a canonical runtime blocker",
            })
        })
        .collect::<Vec<_>>();
    let blocking = if status == "blocked" {
        drift.clone()
    } else {
        Vec::new()
    };
    SharedRuntimeObservation {
        schema: "cynic.network_mesh.runtime_observation.v1",
        source: "cynic-node observe",
        status,
        severity,
        blockers: blockers.clone(),
        missing: blockers,
        drift,
        blocking,
        wireguard_transport: serde_json::to_value(wireguard_transport)
            .unwrap_or_else(|_| serde_json::json!({})),
        dns_projection: serde_json::to_value(dns_projection)
            .unwrap_or_else(|_| serde_json::json!({})),
    }
}
