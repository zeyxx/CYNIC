//! Canonical network plane types for CYNIC.
//!
//! A plane is an address-domain intent. It is not an interface, namespace,
//! route, tunnel, or service publication by itself.
//!
//! This module is 100% pure: no I/O, no filesystem, no network calls.
//! Side-effects stay behind explicit ports in `cynic-network` or `cynic-node`.

use serde::{Deserialize, Serialize};

/// Canonical network planes for CYNIC.
///
/// Plane names are functional, English, non-metaphorical.
/// The name describes what the plane does, not what it evokes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plane {
    /// CYNIC service endpoints — kernel, builder, storage, forgejo.
    Service,
    /// Bare-metal management and hypervisor control.
    Admin,
    /// WireGuard encrypted full-mesh between nodes.
    Mesh,
}

impl Plane {
    /// All canonical planes in canonical order.
    pub const ALL: [Plane; 3] = [Plane::Service, Plane::Admin, Plane::Mesh];

    /// Parse a plane from its canonical string name.
    pub fn from_name(name: &str) -> Option<Plane> {
        match name {
            "service" => Some(Plane::Service),
            "admin" => Some(Plane::Admin),
            "mesh" => Some(Plane::Mesh),
            _ => None,
        }
    }

    /// Canonical string name for this plane.
    pub fn name(&self) -> &'static str {
        match self {
            Plane::Service => "service",
            Plane::Admin => "admin",
            Plane::Mesh => "mesh",
        }
    }

    /// Default CIDR for this plane.
    ///
    /// These are defaults only. Runtime configuration MUST come from
    /// `infra/registry.json` or `infra/network/topology.json`.
    pub fn default_cidr(&self) -> &'static str {
        match self {
            Plane::Service => "10.10.10.0/24",
            Plane::Admin => "192.168.100.0/24",
            Plane::Mesh => "10.20.0.0/24",
        }
    }

    /// DNS zone for this plane, if any.
    pub fn dns_zone(&self) -> Option<&'static str> {
        match self {
            Plane::Service => Some("service.cynic"),
            Plane::Admin => None,
            Plane::Mesh => None,
        }
    }

    /// Federation scope for this plane.
    pub fn federation_scope(&self) -> &'static str {
        match self {
            Plane::Service => "canonical_service_plane",
            Plane::Admin => "administration_only",
            Plane::Mesh => "encrypted_transport_plane",
        }
    }

    /// Purpose description for this plane.
    pub fn purpose(&self) -> &'static str {
        match self {
            Plane::Service => "CYNIC service endpoints — kernel, builder, storage, forgejo",
            Plane::Admin => "bare-metal management and hypervisor control",
            Plane::Mesh => "WireGuard encrypted full-mesh between nodes",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_names_are_functional() {
        assert_eq!(Plane::Service.name(), "service");
        assert_eq!(Plane::Admin.name(), "admin");
        assert_eq!(Plane::Mesh.name(), "mesh");
    }

    #[test]
    fn plane_from_name_round_trips() {
        for plane in Plane::ALL {
            assert_eq!(Plane::from_name(plane.name()), Some(plane));
        }
    }

    #[test]
    fn plane_from_name_rejects_unknown() {
        assert_eq!(Plane::from_name("synapse"), None);
        assert_eq!(Plane::from_name("management"), None);
        assert_eq!(Plane::from_name("overlay"), None);
        assert_eq!(Plane::from_name(""), None);
    }

    #[test]
    fn plane_default_cidrs_are_canonical() {
        assert_eq!(Plane::Service.default_cidr(), "10.10.10.0/24");
        assert_eq!(Plane::Admin.default_cidr(), "192.168.100.0/24");
        assert_eq!(Plane::Mesh.default_cidr(), "10.20.0.0/24");
    }

    #[test]
    fn plane_dns_zones_are_canonical() {
        assert_eq!(Plane::Service.dns_zone(), Some("service.cynic"));
        assert_eq!(Plane::Admin.dns_zone(), None);
        assert_eq!(Plane::Mesh.dns_zone(), None);
    }

    #[test]
    fn plane_federation_scopes_are_canonical() {
        assert_eq!(Plane::Service.federation_scope(), "canonical_service_plane");
        assert_eq!(Plane::Admin.federation_scope(), "administration_only");
        assert_eq!(Plane::Mesh.federation_scope(), "encrypted_transport_plane");
    }

    #[test]
    fn plane_serializes_to_snake_case() {
        let json = serde_json::to_string(&Plane::Service).unwrap();
        assert_eq!(json, "\"service\"");
        let json = serde_json::to_string(&Plane::Admin).unwrap();
        assert_eq!(json, "\"admin\"");
        let json = serde_json::to_string(&Plane::Mesh).unwrap();
        assert_eq!(json, "\"mesh\"");
    }

    #[test]
    fn plane_deserializes_from_snake_case() {
        let plane: Plane = serde_json::from_str("\"service\"").unwrap();
        assert_eq!(plane, Plane::Service);
        let plane: Plane = serde_json::from_str("\"admin\"").unwrap();
        assert_eq!(plane, Plane::Admin);
        let plane: Plane = serde_json::from_str("\"mesh\"").unwrap();
        assert_eq!(plane, Plane::Mesh);
    }
}
