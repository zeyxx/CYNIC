use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;

use crate::domain::mesh::PeerIdentity;
use crate::network_mesh_plan::model::MeshRegistry;
use crate::ports::mesh_port::MeshStateProvider;

pub struct RegistryMeshStateProvider {
    registry_path: PathBuf,
    self_node_id: String,
}

impl RegistryMeshStateProvider {
    pub fn new(registry_path: PathBuf, self_node_id: String) -> Self {
        Self {
            registry_path,
            self_node_id,
        }
    }
}

impl MeshStateProvider for RegistryMeshStateProvider {
    async fn fetch_known_peers(&self) -> Result<Vec<PeerIdentity>, Box<dyn std::error::Error>> {
        let raw = tokio::fs::read_to_string(&self.registry_path).await?;
        let registry: MeshRegistry = serde_json::from_str(&raw)?;

        let peers = registry
            .wireguard_nodes()
            .iter()
            .filter(|n| n.node_id != self.self_node_id)
            .map(|n| {
                let internal_ip = IpAddr::from_str(&n.overlay_ip).map_err(|e| {
                    format!(
                        "invalid overlay_ip '{}' for node '{}': {}",
                        n.overlay_ip, n.node_id, e
                    )
                })?;
                let endpoint = if n.endpoint.is_empty() {
                    None
                } else {
                    Some(n.endpoint.clone())
                };
                let keepalive_s = registry.wireguard_policy().and_then(|p| p.keepalive_s);
                Ok(PeerIdentity::new(
                    n.public_key.clone(),
                    internal_ip,
                    endpoint,
                    keepalive_s,
                ))
            })
            .collect::<Result<Vec<PeerIdentity>, String>>()?;

        Ok(peers)
    }
}
