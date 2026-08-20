//! Headscale coordination adapter — membership, not transport.
//!
//! This adapter carries into Rust the residual capabilities of the former
//! `organ-headscale` (Python). It is no longer contract-less: it implements
//! `MeshCoordinationPort`, which pins its role to `Coordination`.
//!
//! - `WireGuard` = canonical transport (owns interfaces, keys, routes)
//! - `Headscale` = canonical coordination (owns enrolment, revocation, discovery)
//! - neither owns topology truth; that is the registry's
//!
//! Sovereignty: the control plane is self-hosted (LXC 100, `10.10.10.40:8080`,
//! observed 2026-08-20: `/health` 200, `/api/v1/node` 401 — alive and
//! authenticating). That is what distinguishes it from Tailscale SaaS, which
//! `cynic_protocol::network::admit_mesh_provider` refuses by type. The client
//! protocol is not the problem; a third-party control plane is.
//!
//! It must remain bounded to provider-specific coordination and must never
//! become canonical topology, policy, or runtime truth.

use crate::ports::mesh_coordination_port::{
    MeshCoordinationError, MeshCoordinationPort, MeshJoinGrant, MeshMember,
};
use cynic_host::process_observation::capture_command;
use cynic_host::runtime_layout::current_runtime_layout;
use cynic_protocol::network::{MeshRole, MeshSovereignty};
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

impl MeshCoordinationPort for HeadscaleAdapter {
    fn provider(&self) -> String {
        "headscale".to_string()
    }

    /// Self-hosted control plane: this is the whole reason Headscale is retained
    /// where Tailscale is refused.
    fn sovereignty(&self) -> MeshSovereignty {
        MeshSovereignty::Sovereign
    }

    fn list_members(&self) -> Result<Vec<MeshMember>, MeshCoordinationError> {
        // A control plane we cannot reach has refused nothing, so a transport or
        // spawn failure is Unobservable rather than Refused.
        let raw = self
            .list_nodes()
            .map_err(MeshCoordinationError::Unobservable)?;

        let nodes: Vec<HeadscaleNode> = serde_json::from_str(&raw)
            .map_err(|e| MeshCoordinationError::Malformed(format!("nodes list: {e}")))?;

        Ok(nodes
            .into_iter()
            .map(|n| MeshMember {
                member_id: n.id.to_string(),
                name: n.name,
                addresses: n.ip_addresses,
                provider_reports_online: Some(n.online),
            })
            .collect())
    }

    fn issue_join_grant(&self, user: &str) -> Result<MeshJoinGrant, MeshCoordinationError> {
        let secret = self
            .generate_preauthkey(user)
            .map_err(MeshCoordinationError::Unobservable)?;

        if secret.is_empty() {
            return Err(MeshCoordinationError::Malformed(
                "preauthkey command returned an empty key".to_string(),
            ));
        }

        Ok(MeshJoinGrant {
            secret,
            // The adapter requests a 24h expiry (`-e 24h`) but headscale does not
            // echo the resulting instant back, so the concrete expiry stays an
            // explicit unknown instead of a computed guess.
            expires_at_unix_s: None,
            issued_by_provider: self.provider(),
        })
    }

    fn revoke_member(&self, member_id: &str) -> Result<(), MeshCoordinationError> {
        let id: u64 = member_id.parse().map_err(|_| {
            MeshCoordinationError::Malformed(format!(
                "headscale member ids are numeric, got {member_id:?}"
            ))
        })?;
        self.revoke_node(id)
            .map_err(MeshCoordinationError::Unobservable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_adapter_declares_coordination_and_sovereign() {
        let adapter = HeadscaleAdapter::new("http://10.10.10.40:8080");
        assert_eq!(adapter.provider(), "headscale");
        assert_eq!(adapter.role(), MeshRole::Coordination);
        assert_eq!(adapter.sovereignty(), MeshSovereignty::Sovereign);
    }

    /// The provider binding table and the adapter must agree; a mismatch would
    /// mean the sovereignty gate is judging something other than what runs.
    #[test]
    fn the_adapter_matches_its_protocol_binding() {
        let adapter = HeadscaleAdapter::new("http://10.10.10.40:8080");
        let binding = cynic_protocol::network::admit_mesh_provider(&adapter.provider())
            .expect("self-hosted headscale must be admitted");
        assert_eq!(binding.role, adapter.role());
        assert_eq!(binding.sovereignty, adapter.sovereignty());
    }

    /// Non-numeric ids are rejected before any command is spawned: a malformed
    /// request is not an unreachable control plane.
    #[test]
    fn a_non_numeric_member_id_is_malformed_not_unobservable() {
        let adapter = HeadscaleAdapter::new("http://10.10.10.40:8080");
        match adapter.revoke_member("not-a-number") {
            Err(MeshCoordinationError::Malformed(m)) => assert!(m.contains("numeric"), "{m}"),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }
}
