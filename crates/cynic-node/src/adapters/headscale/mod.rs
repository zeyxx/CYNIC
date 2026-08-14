//! Legacy Headscale Coordination Adapter
//!
//! Cet adaptateur porte en Rust des capacités résiduelles de l'ancien
//! `organ-headscale` (Python).
//! Il ne représente pas l'identité canonique du mesh organisme:
//! - `Synapse` = capacité canonique du mesh
//! - `WireGuard` = transport canonique
//! - `Headscale` = adaptateur transitoire de coordination/découverte compatible
//!   clients Tailscale
//!
//! Il doit rester borné à la coordination provider-specific et ne doit jamais
//! devenir la vérité canonique de topologie, de politique ou de runtime mesh.

use cynic_host::process_observation::capture_command;
use cynic_host::runtime_layout::current_runtime_layout;
use serde::{Deserialize, Serialize};

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

    fn headscale_bin(&self) -> String {
        current_runtime_layout().headscale_bin()
    }

    /// Correspond à `list_nodes` du mesh_contract.py
    pub fn list_nodes(&self) -> Result<String, String> {
        let output = capture_command(&self.headscale_bin(), &["nodes", "list", "-o", "json"])
            .map_err(|e| format!("Failed to list headscale nodes: {}", e))?;

        if !output.success {
            return Err(output.stderr);
        }

        Ok(output.stdout)
    }

    /// Correspond à `generate_preauthkey` du mesh_contract.py
    pub fn generate_preauthkey(&self, user: &str) -> Result<String, String> {
        let output = capture_command(
            &self.headscale_bin(),
            &["preauthkeys", "create", "-e", "24h", "-u", user],
        )
        .map_err(|e| format!("Failed to generate preauthkey: {}", e))?;

        if !output.success {
            return Err(output.stderr);
        }

        Ok(output.stdout.trim().to_string())
    }

    /// Correspond à `revoke_node` du mesh_contract.py
    pub fn revoke_node(&self, node_id: u64) -> Result<(), String> {
        let node_id_str = node_id.to_string();
        let output = capture_command(
            &self.headscale_bin(),
            &["nodes", "delete", "-i", &node_id_str, "--force"],
        )
        .map_err(|e| format!("Failed to revoke node: {}", e))?;

        if !output.success {
            return Err(output.stderr);
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
