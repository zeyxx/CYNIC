//! Headscale Access Mesh Adapter
//!
//! Cet adaptateur porte en Rust les capacités de l'ancien `organ-headscale` (Python).
//! Il interagit avec le control plane souverain (Headscale) pour lister les nœuds,
//! générer des clés d'authentification et observer la santé du mesh d'accès.

use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct HeadscaleAdapter {
    pub endpoint: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeadscaleNode {
    pub id: u64,
    pub name: String,
    pub ip_addresses: Vec<String>,
    pub online: bool,
}

impl HeadscaleAdapter {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
        }
    }

    /// Correspond à `list_nodes` du mesh_contract.py
    pub fn list_nodes(&self) -> Result<String, String> {
        let output = Command::new("headscale")
            .arg("nodes")
            .arg("list")
            .arg("-o")
            .arg("json")
            .output()
            .map_err(|e| format!("Failed to list headscale nodes: {}", e))?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Correspond à `generate_preauthkey` du mesh_contract.py
    pub fn generate_preauthkey(&self, user: &str) -> Result<String, String> {
        let output = Command::new("headscale")
            .arg("preauthkeys")
            .arg("create")
            .arg("-e")
            .arg("24h")
            .arg("-u")
            .arg(user)
            .output()
            .map_err(|e| format!("Failed to generate preauthkey: {}", e))?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Correspond à `revoke_node` du mesh_contract.py
    pub fn revoke_node(&self, node_id: u64) -> Result<(), String> {
        let output = Command::new("headscale")
            .arg("nodes")
            .arg("delete")
            .arg("-i")
            .arg(node_id.to_string())
            .arg("--force")
            .output()
            .map_err(|e| format!("Failed to revoke node: {}", e))?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }

        Ok(())
    }

    /// Correspond à `observe_node_liveness` du mesh_contract.py
    pub fn observe_node_liveness(&self, node_name: &str) -> Result<bool, String> {
        // En json brut, on filtre
        let json_data = self.list_nodes()?;
        let nodes: Vec<HeadscaleNode> = serde_json::from_str(&json_data)
            .map_err(|e| format!("Failed to parse nodes: {}", e))?;

        for node in nodes {
            if node.name == node_name {
                return Ok(node.online);
            }
        }

        Err(format!("Node {} not found", node_name))
    }
}
