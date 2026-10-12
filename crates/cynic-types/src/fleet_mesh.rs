//! Fleet mesh domain port — pure types and trait, Zero-I/O.
//!
//! This port abstracts the fleet mesh (Headscale, WireGuard, or any
//! future implementation) behind a single trait. Adapters live in
//! `cynic-network` (I/O), never here.
//!
//! Types are canonical: `cynic-network` re-exports them from here,
//! it does NOT define its own mesh types.

use serde::{Deserialize, Serialize};

// FleetNetworkNode and FleetNetworkReport remain in cynic-network —
// they reference NetworkReport (I/O observation type).

/// A node in the fleet mesh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshNode {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub online: bool,
    pub last_seen: Option<String>,
}

/// A route in the fleet mesh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshRoute {
    pub id: String,
    pub prefix: String,
    pub enabled: bool,
    pub node_id: String,
}

/// Pre-authentication key specification for dynamic node admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreAuthKeySpec {
    pub user: String,
    pub reusable: bool,
    pub ephemeral: bool,
    pub expiration: Option<String>,
    pub acl_tags: Vec<String>,
}

/// Created pre-authentication key returned by mesh coordinator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreAuthKey {
    pub id: String,
    pub key: String,
    pub user: String,
    pub reusable: bool,
    pub ephemeral: bool,
    pub used: bool,
    pub expiration: Option<String>,
}

/// Errors that can occur when interacting with the fleet mesh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshError {
    NodeNotFound(String),
    RouteNotFound(String),
    ApiError(String),
    Unauthorized,
    NetworkUnreachable,
}

/// Pure port for fleet mesh operations.
///
/// Implementations must be I/O-free in their signatures — all side effects
/// live in the adapter (e.g. `HeadscaleClient` in `cynic-network`).
pub trait FleetMeshPort {
    /// List all nodes in the fleet mesh.
    fn list_nodes(&self) -> Result<Vec<MeshNode>, MeshError>;

    /// Enroll a new node into the fleet mesh.
    fn enroll_node(&self, node: &MeshNode) -> Result<(), MeshError>;

    /// Remove a node from the fleet mesh.
    fn remove_node(&self, node_id: &str) -> Result<(), MeshError>;

    /// List all routes in the fleet mesh.
    fn list_routes(&self) -> Result<Vec<MeshRoute>, MeshError>;

    /// Enable a route in the fleet mesh.
    fn enable_route(&self, route_id: &str) -> Result<(), MeshError>;

    /// Disable a route in the fleet mesh.
    fn disable_route(&self, route_id: &str) -> Result<(), MeshError>;

    /// Create a dynamic pre-authentication key for admitting a new node.
    fn create_preauth_key(&self, _spec: &PreAuthKeySpec) -> Result<PreAuthKey, MeshError> {
        Err(MeshError::ApiError(
            "preauth key creation not implemented by adapter".to_string(),
        ))
    }

    /// Expire a pre-authentication key to prevent further admissions.
    fn expire_preauth_key(&self, _user: &str, _key: &str) -> Result<(), MeshError> {
        Err(MeshError::ApiError(
            "preauth key expiration not implemented by adapter".to_string(),
        ))
    }
}

/// Canonical mesh plan — desired state of the fleet mesh.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshPlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub provider: String,
    pub registry_path: String,
    pub service_plane: String,
    pub cidr: Option<String>,
    pub dns_zone: Option<String>,
    pub desired_peer_count: usize,
    pub desired_peers: Vec<NetworkMeshPeer>,
    pub routes: Vec<NetworkMeshRoute>,
    pub dns: Vec<NetworkMeshDnsRecord>,
    pub acl: Vec<NetworkMeshAclRule>,
    pub actuator_required: bool,
    pub can_apply: bool,
    pub apply_requires: Vec<&'static str>,
}

/// A peer in the fleet mesh.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshPeer {
    pub id: String,
    pub role: Option<String>,
    pub status: Option<String>,
    pub address: String,
    pub local_address: Option<String>,
    pub dns_names: Vec<String>,
    pub services: Vec<NetworkMeshService>,
}

/// A service exposed by a mesh peer.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshService {
    pub name: String,
    pub port: Option<u16>,
    pub protocol: String,
}

/// A route in the fleet mesh.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshRoute {
    pub destination: String,
    pub via_peer: String,
    pub provider: String,
}

/// A DNS record in the fleet mesh.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshDnsRecord {
    pub name: String,
    pub address: String,
}

/// An ACL rule in the fleet mesh.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshAclRule {
    pub id: String,
    pub source: String,
    pub destination: String,
    pub port: Option<u16>,
    pub protocol: String,
    pub action: String,
}

/// Assessment of WireGuard mesh peer status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireguardMeshAssessment {
    pub total_peers: usize,
    pub ok_peers: usize,
    pub stale_peers: usize,
    pub dead_peers: usize,
    pub never_seen_peers: usize,
}

// TODO: Fleet mesh is NOT a HostSubstrateDomain (principal/privilege/packages/
// filesystem/services). The mesh is a fleet-level concern, not a host substrate
// concern. A proper FleetSubstrateDomain variant must be added to shared.rs
// before mesh_contract() can be implemented. Not faking it with Services.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_node_is_pure_data() {
        let node = MeshNode {
            id: "1".to_string(),
            name: "ubuntu".to_string(),
            ip: "100.64.0.1".to_string(),
            online: false,
            last_seen: Some("2026-06-28T21:54:16Z".to_string()),
        };
        assert_eq!(node.id, "1");
        assert_eq!(node.name, "ubuntu");
        assert!(!node.online);
    }

    #[test]
    fn mesh_route_is_pure_data() {
        let route = MeshRoute {
            id: "1".to_string(),
            prefix: "100.64.0.0/10".to_string(),
            enabled: true,
            node_id: "1".to_string(),
        };
        assert_eq!(route.prefix, "100.64.0.0/10");
        assert!(route.enabled);
    }

    #[test]
    fn mesh_error_is_pure_data() {
        let err = MeshError::NodeNotFound("1".to_string());
        assert_eq!(err, MeshError::NodeNotFound("1".to_string()));
    }

    #[test]
    fn mesh_error_unauthorized() {
        let err = MeshError::Unauthorized;
        assert_eq!(err, MeshError::Unauthorized);
    }

    #[test]
    fn mesh_error_network_unreachable() {
        let err = MeshError::NetworkUnreachable;
        assert_eq!(err, MeshError::NetworkUnreachable);
    }

    #[test]
    fn network_mesh_plan_is_pure_data() {
        let plan = NetworkMeshPlan {
            schema: "cynic.network_mesh_plan.v1",
            status: "planned",
            provider: "headscale".to_string(),
            registry_path: "/etc/cynic/registry.json".to_string(),
            service_plane: "service".to_string(),
            cidr: Some("100.64.0.0/10".to_string()),
            dns_zone: Some("cynic.ts".to_string()),
            desired_peer_count: 3,
            desired_peers: vec![],
            routes: vec![],
            dns: vec![],
            acl: vec![],
            actuator_required: false,
            can_apply: true,
            apply_requires: vec![],
        };
        assert_eq!(plan.desired_peer_count, 3);
        assert_eq!(plan.provider, "headscale");
    }

    #[test]
    fn wireguard_mesh_assessment_is_pure_data() {
        let assessment = WireguardMeshAssessment {
            total_peers: 3,
            ok_peers: 2,
            stale_peers: 1,
            dead_peers: 0,
            never_seen_peers: 0,
        };
        assert_eq!(assessment.total_peers, 3);
        assert_eq!(assessment.ok_peers, 2);
    }

    // TODO: mesh_contract() test deferred until FleetSubstrateDomain is added
}
