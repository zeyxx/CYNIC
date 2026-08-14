use super::*;

fn caps() -> NodeRuntimeCapabilities {
    NodeRuntimeCapabilities {
        provider_fallback_active: true,
        synapse_route_active: false,
        wireguard_active: false,
        critical_dns_ok: true,
        critical_dns_canonical: true,
        critical_tcp_open: true,
    }
}

#[test]
fn classifies_provider_fallback_as_alive_but_not_mesh() {
    let verdict = classify_runtime(caps());

    assert_eq!(verdict.status, NodeRuntimeStatus::ProviderFallbackActive);
    assert_eq!(verdict.severity, NodeRuntimeSeverity::Warning);
    assert_eq!(
        verdict.blockers,
        vec![
            NodeRuntimeBlocker::SynapseRouteMissing,
            NodeRuntimeBlocker::WireguardMeshAbsent
        ]
    );
    assert_eq!(
        verdict
            .remediation
            .iter()
            .map(|item| item.action.clone())
            .collect::<Vec<_>>(),
        vec![
            NodeRuntimeAction::ActivateSynapseRoute,
            NodeRuntimeAction::ActivateWireguardMesh
        ]
    );
    assert!(
        verdict
            .remediation
            .iter()
            .all(|item| item.canonical_surface == SURFACE_NETWORK_MESH_STATUS)
    );
}

#[test]
fn classifies_full_mesh_as_active() {
    let mut capabilities = caps();
    capabilities.synapse_route_active = true;
    capabilities.wireguard_active = true;

    let verdict = classify_runtime(capabilities);

    assert_eq!(verdict.status, NodeRuntimeStatus::MeshActive);
    assert_eq!(verdict.severity, NodeRuntimeSeverity::Clear);
    assert!(verdict.blockers.is_empty());
    assert!(verdict.remediation.is_empty());
}

#[test]
fn classifies_missing_connectivity_as_blocked() {
    let mut capabilities = caps();
    capabilities.critical_tcp_open = false;

    let verdict = classify_runtime(capabilities);

    assert_eq!(verdict.status, NodeRuntimeStatus::Blocked);
    assert_eq!(verdict.severity, NodeRuntimeSeverity::Critical);
    assert!(
        verdict
            .blockers
            .contains(&NodeRuntimeBlocker::CriticalTcpConnectivityFailed)
    );
}

#[test]
fn classifies_noncanonical_dns_as_warning_with_repair_path() {
    let mut capabilities = caps();
    capabilities.synapse_route_active = true;
    capabilities.wireguard_active = true;
    capabilities.critical_dns_canonical = false;

    let verdict = classify_runtime(capabilities);

    assert_eq!(verdict.status, NodeRuntimeStatus::MeshActive);
    assert_eq!(verdict.severity, NodeRuntimeSeverity::Warning);
    assert!(
        verdict
            .blockers
            .contains(&NodeRuntimeBlocker::CanonicalDnsProjectionDrift)
    );
    assert!(
        verdict
            .remediation
            .iter()
            .any(|item| item.action == NodeRuntimeAction::RestoreCanonicalDnsProjection)
    );
    assert!(
        verdict
            .remediation
            .iter()
            .any(|item| item.canonical_surface == SURFACE_NETWORK_MESH_DNS_ACTIVATION_STATUS)
    );
}

#[test]
fn remediation_surfaces_match_infrastructure_contract_operation_names() {
    let verdict = classify_runtime(NodeRuntimeCapabilities {
        provider_fallback_active: false,
        synapse_route_active: false,
        wireguard_active: false,
        critical_dns_ok: false,
        critical_dns_canonical: false,
        critical_tcp_open: false,
    });

    let surfaces = verdict
        .remediation
        .iter()
        .map(|item| item.canonical_surface)
        .collect::<Vec<_>>();

    assert!(surfaces.contains(&SURFACE_NETWORK_MESH_PROVIDER_FALLBACK_STATUS));
    assert!(surfaces.contains(&SURFACE_NETWORK_MESH_DNS_ACTIVATION_STATUS));
    assert!(surfaces.contains(&SURFACE_NETWORK_MESH_STATUS));
    assert!(surfaces.contains(&SURFACE_FLEET_NODE_STATUS));
}
