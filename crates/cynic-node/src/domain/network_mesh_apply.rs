use cynic_protocol::NETWORK_MESH_PLAN_SCHEMA;

use std::collections::BTreeSet;
use std::net::IpAddr;

use serde::Deserialize;

#[derive(Debug, Clone)]
pub(crate) struct MeshApplyBundle {
    pub(crate) manifest: MeshBundleManifest,
    pub(crate) plan: MeshPlanFile,
    pub(crate) complete: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MeshBundleManifest {
    pub(crate) schema: String,
    pub(crate) status: String,
    pub(crate) provider: String,
    pub(crate) bundle_files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MeshPlanFile {
    pub(crate) schema: String,
    pub(crate) status: String,
    pub(crate) provider: String,
    pub(crate) cidr: Option<String>,
    pub(crate) desired_peer_count: usize,
    pub(crate) desired_peers: Vec<MeshPlanPeerFile>,
    pub(crate) routes: Vec<MeshPlanRouteFile>,
    pub(crate) dns: Vec<MeshPlanDnsRecordFile>,
    pub(crate) acl: Vec<MeshPlanAclRuleFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MeshPlanPeerFile {
    pub(crate) id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MeshPlanRouteFile {
    pub(crate) destination: String,
    pub(crate) via_peer: String,
    pub(crate) provider: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MeshPlanDnsRecordFile {
    pub(crate) name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MeshPlanAclRuleFile {}

pub(crate) fn validate_mesh_apply_bundle(
    manifest: &MeshBundleManifest,
    plan: &MeshPlanFile,
) -> Result<(), String> {
    if manifest.schema != "cynic.network_mesh_bundle.v1" {
        return Err(format!(
            "unsupported mesh bundle schema: {}",
            manifest.schema
        ));
    }
    if !matches!(manifest.status.as_str(), "planned" | "insufficient_peers") {
        return Err(format!(
            "mesh bundle has unknown planning status: {}",
            manifest.status
        ));
    }
    if plan.schema != NETWORK_MESH_PLAN_SCHEMA || plan.status != manifest.status {
        return Err("mesh bundle manifest and plan schema/status must match".to_string());
    }
    if manifest.provider.trim().is_empty() || manifest.provider != plan.provider {
        return Err("mesh bundle manifest and plan provider must match".to_string());
    }
    if plan.desired_peer_count != plan.desired_peers.len() {
        return Err("mesh plan desired_peer_count does not match desired_peers".to_string());
    }

    let peer_ids = plan
        .desired_peers
        .iter()
        .map(|peer| peer.id.as_str())
        .collect::<BTreeSet<_>>();
    if peer_ids.len() != plan.desired_peers.len()
        || peer_ids.iter().any(|peer_id| peer_id.trim().is_empty())
    {
        return Err("mesh plan peer identities must be unique and non-empty".to_string());
    }
    for route in &plan.routes {
        validate_ip_network(&route.destination)?;
        if route.provider != plan.provider {
            return Err(format!(
                "mesh route provider mismatch for {}",
                route.destination
            ));
        }
        if !peer_ids.contains(route.via_peer.as_str()) {
            return Err(format!(
                "mesh route {} references unknown peer {}",
                route.destination, route.via_peer
            ));
        }
    }
    if let Some(cidr) = &plan.cidr {
        validate_ip_network(cidr)?;
    }
    Ok(())
}

fn validate_ip_network(value: &str) -> Result<(), String> {
    let (address, prefix) = value
        .split_once('/')
        .ok_or_else(|| format!("mesh network must use CIDR notation: {value}"))?;
    let address = address
        .parse::<IpAddr>()
        .map_err(|_| format!("mesh network has invalid IP address: {value}"))?;
    let prefix = prefix
        .parse::<u8>()
        .map_err(|_| format!("mesh network has invalid prefix: {value}"))?;
    let max_prefix = if address.is_ipv4() { 32 } else { 128 };
    if prefix > max_prefix {
        return Err(format!("mesh network prefix is out of range: {value}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        MeshBundleManifest, MeshPlanFile, MeshPlanPeerFile, MeshPlanRouteFile,
        validate_mesh_apply_bundle,
    };

    fn bundle() -> (MeshBundleManifest, MeshPlanFile) {
        (
            MeshBundleManifest {
                schema: "cynic.network_mesh_bundle.v1".to_string(),
                status: "planned".to_string(),
                provider: "wireguard".to_string(),
                bundle_files: vec!["mesh-plan.json".to_string()],
            },
            MeshPlanFile {
                schema: NETWORK_MESH_PLAN_SCHEMA.to_string(),
                status: "planned".to_string(),
                provider: "wireguard".to_string(),
                cidr: Some("10.30.0.0/24".to_string()),
                desired_peer_count: 1,
                desired_peers: vec![MeshPlanPeerFile {
                    id: "node-a".to_string(),
                }],
                routes: vec![MeshPlanRouteFile {
                    destination: "10.30.0.10/32".to_string(),
                    via_peer: "node-a".to_string(),
                    provider: "wireguard".to_string(),
                }],
                dns: Vec::new(),
                acl: Vec::new(),
            },
        )
    }

    #[test]
    fn valid_bundle_is_admitted_by_domain_policy() {
        let (manifest, plan) = bundle();

        assert!(validate_mesh_apply_bundle(&manifest, &plan).is_ok());
    }

    #[test]
    fn route_argument_injection_is_rejected_before_command_projection() {
        let (manifest, mut plan) = bundle();
        plan.routes[0].destination = "--help".to_string();

        let error = validate_mesh_apply_bundle(&manifest, &plan).expect_err("refusal");

        assert!(error.contains("CIDR notation"));
    }

    #[test]
    fn route_cannot_target_an_unknown_peer() {
        let (manifest, mut plan) = bundle();
        plan.routes[0].via_peer = "unknown".to_string();

        let error = validate_mesh_apply_bundle(&manifest, &plan).expect_err("refusal");

        assert!(error.contains("unknown peer"));
    }
}
