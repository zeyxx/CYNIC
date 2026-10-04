//! Fleet mesh domain port — pure types and trait, Zero-I/O.
//!
//! This port abstracts the fleet mesh (Headscale, WireGuard, or any
//! future implementation) behind a single trait. Adapters live in
//! `cynic-network` (I/O), never here.

use serde::{Deserialize, Serialize};

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
}

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
}
