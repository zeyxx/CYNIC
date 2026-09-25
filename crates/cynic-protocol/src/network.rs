use serde::Serialize;

pub const NETWORK_MESH_PLAN_SCHEMA: &str = "cynic.network_mesh_plan.v1";

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

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshService {
    pub name: String,
    pub port: Option<u16>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshRoute {
    pub destination: String,
    pub via_peer: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshDnsRecord {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshAclRule {
    pub id: String,
    pub source: String,
    pub destination: String,
    pub port: Option<u16>,
    pub protocol: String,
    pub action: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_mesh_plan_schema_is_stable() {
        let plan = NetworkMeshPlan {
            schema: NETWORK_MESH_PLAN_SCHEMA,
            status: "planned",
            provider: "wireguard".to_string(),
            registry_path: "registry.json".to_string(),
            service_plane: "synapse".to_string(),
            cidr: Some("10.0.0.0/24".to_string()),
            dns_zone: Some("mesh.test".to_string()),
            desired_peer_count: 0,
            desired_peers: Vec::new(),
            routes: Vec::new(),
            dns: Vec::new(),
            acl: Vec::new(),
            actuator_required: true,
            can_apply: false,
            apply_requires: vec!["network_actuator_bundle"],
        };
        assert_eq!(plan.schema, "cynic.network_mesh_plan.v1");
        assert!(!plan.can_apply);
    }
}
