use super::{
    NodeRuntimeAction, NodeRuntimeBlocker, NodeRuntimeCapabilities, NodeRuntimeRemediation,
    NodeRuntimeSeverity, NodeRuntimeStatus, NodeRuntimeVerdict,
    SURFACE_FLEET_NODE_STATUS, SURFACE_NETWORK_MESH_DNS_ACTIVATION_STATUS,
    SURFACE_NETWORK_MESH_PROVIDER_FALLBACK_HOSTS, SURFACE_NETWORK_MESH_PROVIDER_FALLBACK_STATUS,
    SURFACE_NETWORK_MESH_STATUS,
};

pub fn classify_runtime(capabilities: NodeRuntimeCapabilities) -> NodeRuntimeVerdict {
    let mut blockers = Vec::new();
    if !capabilities.provider_fallback_active {
        blockers.push(NodeRuntimeBlocker::ProviderFallbackRouteMissing);
    }
    if !capabilities.critical_dns_ok {
        blockers.push(NodeRuntimeBlocker::CriticalDnsResolutionFailed);
    }
    if !capabilities.critical_dns_canonical {
        blockers.push(NodeRuntimeBlocker::CanonicalDnsProjectionDrift);
    }
    if !capabilities.critical_tcp_open {
        blockers.push(NodeRuntimeBlocker::CriticalTcpConnectivityFailed);
    }
    if !capabilities.synapse_route_active {
        blockers.push(NodeRuntimeBlocker::SynapseRouteMissing);
    }
    if !capabilities.wireguard_active {
        blockers.push(NodeRuntimeBlocker::WireguardMeshAbsent);
    }

    let status = if capabilities.critical_tcp_open
        && capabilities.critical_dns_ok
        && (capabilities.provider_fallback_active || capabilities.synapse_route_active)
    {
        if capabilities.wireguard_active && capabilities.synapse_route_active {
            NodeRuntimeStatus::MeshActive
        } else {
            NodeRuntimeStatus::ProviderFallbackActive
        }
    } else {
        NodeRuntimeStatus::Blocked
    };

    let severity = match status {
        NodeRuntimeStatus::MeshActive if capabilities.critical_dns_canonical => {
            NodeRuntimeSeverity::Clear
        }
        NodeRuntimeStatus::MeshActive => NodeRuntimeSeverity::Warning,
        NodeRuntimeStatus::ProviderFallbackActive => NodeRuntimeSeverity::Warning,
        NodeRuntimeStatus::Blocked => NodeRuntimeSeverity::Critical,
    };

    let remediation = remediation_for_blockers(&blockers);

    NodeRuntimeVerdict {
        status,
        severity,
        capabilities,
        blockers,
        remediation,
    }
}

fn remediation_for_blockers(blockers: &[NodeRuntimeBlocker]) -> Vec<NodeRuntimeRemediation> {
    blockers
        .iter()
        .map(|blocker| match blocker {
            NodeRuntimeBlocker::ProviderFallbackRouteMissing => NodeRuntimeRemediation {
                action: NodeRuntimeAction::RestoreProviderFallbackRoute,
                mutates_host: true,
                requires_root: true,
                canonical_surface: SURFACE_NETWORK_MESH_PROVIDER_FALLBACK_STATUS,
                fallback_surface: "ip route replace 10.10.10.0/24 via <registry.provider_gateway>",
                reason: "node cannot reach provider-plane services while canonical Synapse is absent",
            },
            NodeRuntimeBlocker::CriticalDnsResolutionFailed => NodeRuntimeRemediation {
                action: NodeRuntimeAction::RestoreCriticalDns,
                mutates_host: true,
                requires_root: true,
                canonical_surface: SURFACE_NETWORK_MESH_DNS_ACTIVATION_STATUS,
                fallback_surface: SURFACE_NETWORK_MESH_PROVIDER_FALLBACK_HOSTS,
                reason: "critical service names do not resolve on this node",
            },
            NodeRuntimeBlocker::CanonicalDnsProjectionDrift => NodeRuntimeRemediation {
                action: NodeRuntimeAction::RestoreCanonicalDnsProjection,
                mutates_host: true,
                requires_root: true,
                canonical_surface: SURFACE_NETWORK_MESH_DNS_ACTIVATION_STATUS,
                fallback_surface: "python3 organs/fleet/src/entrypoints/cli.py dns bundle",
                reason: "critical service names resolve, but not to the canonical Synapse IPs declared in registry",
            },
            NodeRuntimeBlocker::CriticalTcpConnectivityFailed => NodeRuntimeRemediation {
                action: NodeRuntimeAction::RestoreCriticalTcp,
                mutates_host: false,
                requires_root: false,
                canonical_surface: SURFACE_FLEET_NODE_STATUS,
                fallback_surface: "ssh/curl direct provider-plane probe",
                reason: "at least one critical control-plane socket is unreachable",
            },
            NodeRuntimeBlocker::SynapseRouteMissing => NodeRuntimeRemediation {
                action: NodeRuntimeAction::ActivateSynapseRoute,
                mutates_host: true,
                requires_root: true,
                canonical_surface: SURFACE_NETWORK_MESH_STATUS,
                fallback_surface: "python3 organs/fleet/src/entrypoints/cli.py network-actuator install-plan",
                reason: "canonical organism mesh route is absent",
            },
            NodeRuntimeBlocker::WireguardMeshAbsent => NodeRuntimeRemediation {
                action: NodeRuntimeAction::ActivateWireguardMesh,
                mutates_host: true,
                requires_root: true,
                canonical_surface: SURFACE_NETWORK_MESH_STATUS,
                fallback_surface: "python3 organs/fleet/src/entrypoints/cli.py network-actuator install-plan",
                reason: "WireGuard runtime has no observed peer/interface state",
            },
        })
        .collect()
}
