<<<<<<< HEAD
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
//! `cynic_network::admit_mesh_provider` refuses by type. The client
//! protocol is not the problem; a third-party control plane is.
//!
//! It must remain bounded to provider-specific coordination and must never
//! become canonical topology, policy, or runtime truth.

use crate::ports::mesh_coordination_port::{
    MeshCoordinationError, MeshCoordinationPort, MeshJoinGrant, MeshMember,
};
use cynic_host::process_observation::capture_command;
use cynic_host::runtime_layout::current_runtime_layout;
#[cfg(test)]
use cynic_network::MeshRole;
use cynic_network::MeshSovereignty;
=======
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
>>>>>>> github/main
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct HeadscaleAdapter {
    pub endpoint: String,
<<<<<<< HEAD
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadscaleHealth {
    pub healthy: bool,
    pub endpoint: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeadscaleNode {
    #[serde(deserialize_with = "deserialize_id")]
    pub id: u64,
    pub name: String,
    #[serde(alias = "ipAddresses", default)]
    pub ip_addresses: Vec<String>,
    #[serde(default)]
    pub online: bool,
}

fn deserialize_id<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Id {
        Num(u64),
        Str(String),
    }
    match Id::deserialize(deserializer)? {
        Id::Num(n) => Ok(n),
        Id::Str(s) => s.parse::<u64>().map_err(serde::de::Error::custom),
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum HeadscaleNodeListResponse {
    Wrapped { nodes: Vec<HeadscaleNode> },
    Array(Vec<HeadscaleNode>),
}

pub fn parse_nodes_json(raw: &str) -> Result<Vec<HeadscaleNode>, String> {
    let parsed: HeadscaleNodeListResponse =
        serde_json::from_str(raw).map_err(|e| format!("Failed to parse nodes JSON: {e}"))?;
    match parsed {
        HeadscaleNodeListResponse::Wrapped { nodes } => Ok(nodes),
        HeadscaleNodeListResponse::Array(nodes) => Ok(nodes),
    }
}

#[derive(Debug, Deserialize)]
struct PreAuthKeyResponse {
    #[serde(alias = "preAuthKey")]
    pre_auth_key: Option<PreAuthKeyInner>,
    key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PreAuthKeyInner {
    key: String,
}

pub fn parse_preauthkey_response(raw: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<PreAuthKeyResponse>(raw) {
        if let Some(inner) = parsed.pre_auth_key {
            return inner.key;
        }
        if let Some(k) = parsed.key {
            return k;
        }
    }
    raw.trim().to_string()
}

=======
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeadscaleNode {
    pub id: u64,
    pub name: String,
    pub ip_addresses: Vec<String>,
    pub online: bool,
}

>>>>>>> github/main
impl HeadscaleAdapter {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
<<<<<<< HEAD
            api_key: None,
        }
    }

    pub fn with_api_key(endpoint: &str, api_key: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            api_key: Some(api_key.into()),
=======
>>>>>>> github/main
        }
    }

    fn headscale_bin(&self) -> String {
        current_runtime_layout().headscale_bin()
    }

<<<<<<< HEAD
    fn is_http_endpoint(&self) -> bool {
        self.endpoint.starts_with("http://") || self.endpoint.starts_with("https://")
    }

    /// Probe Headscale `/health` endpoint (public, unauthenticated).
    pub fn check_health(&self) -> Result<HeadscaleHealth, MeshCoordinationError> {
        if self.is_http_endpoint() {
            let url = format!("{}/health", self.endpoint.trim_end_matches('/'));
            let output = capture_command("curl", &["-s", "-m", "3", &url])
                .map_err(|e| MeshCoordinationError::Unobservable(e.to_string()))?;
            if !output.success {
                return Err(MeshCoordinationError::Unobservable(format!(
                    "health check failed with exit code {:?}: {}",
                    output.exit_code, output.stderr
                )));
            }
            let healthy =
                output.stdout.contains("\"status\":\"pass\"") || output.stdout.contains("\"pass\"");
            Ok(HeadscaleHealth {
                healthy,
                endpoint: self.endpoint.clone(),
                status: if healthy {
                    "pass".to_string()
                } else {
                    "fail".to_string()
                },
            })
        } else {
            let output = capture_command(&self.headscale_bin(), &["version"])
                .map_err(|e| MeshCoordinationError::Unobservable(e.to_string()))?;
            Ok(HeadscaleHealth {
                healthy: output.success,
                endpoint: self.endpoint.clone(),
                status: if output.success {
                    "pass".to_string()
                } else {
                    "fail".to_string()
                },
            })
        }
    }

    /// List nodes either via REST `/api/v1/node` or local CLI.
    pub fn list_nodes(&self) -> Result<String, String> {
        if self.is_http_endpoint() {
            let url = format!("{}/api/v1/node", self.endpoint.trim_end_matches('/'));
            let mut args = vec!["-s", "-m", "5"];
            let auth_header;
            if let Some(key) = &self.api_key {
                auth_header = format!("Authorization: Bearer {key}");
                args.push("-H");
                args.push(&auth_header);
            }
            args.push(&url);
            let output = capture_command("curl", &args)
                .map_err(|e| format!("Failed to curl headscale nodes: {e}"))?;
            if !output.success {
                return Err(format!("curl error: {}", output.stderr));
            }
            Ok(output.stdout)
        } else {
            let output = capture_command(&self.headscale_bin(), &["nodes", "list", "-o", "json"])
                .map_err(|e| format!("Failed to list headscale nodes: {}", e))?;
            if !output.success {
                return Err(output.stderr);
            }
            Ok(output.stdout)
        }
    }

    /// Generate preauthkey either via REST `/api/v1/preauthkey` or local CLI.
    pub fn generate_preauthkey(&self, user: &str) -> Result<String, String> {
        if self.is_http_endpoint() {
            let url = format!("{}/api/v1/preauthkey", self.endpoint.trim_end_matches('/'));
            let body = format!(
                r#"{{"user":"{}","reusable":false,"ephemeral":false,"expiration":"24h"}}"#,
                user
            );
            let mut args = vec![
                "-s",
                "-m",
                "5",
                "-X",
                "POST",
                "-H",
                "Content-Type: application/json",
            ];
            let auth_header;
            if let Some(key) = &self.api_key {
                auth_header = format!("Authorization: Bearer {key}");
                args.push("-H");
                args.push(&auth_header);
            }
            args.push("-d");
            args.push(&body);
            args.push(&url);
            let output = capture_command("curl", &args)
                .map_err(|e| format!("Failed to curl preauthkey: {e}"))?;
            if !output.success {
                return Err(format!("curl error: {}", output.stderr));
            }
            Ok(parse_preauthkey_response(&output.stdout))
        } else {
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
    }

    /// Revoke node either via REST `/api/v1/node/{id}` or local CLI.
    pub fn revoke_node(&self, node_id: u64) -> Result<(), String> {
        let node_id_str = node_id.to_string();
        if self.is_http_endpoint() {
            let url = format!(
                "{}/api/v1/node/{}",
                self.endpoint.trim_end_matches('/'),
                node_id_str
            );
            let mut args = vec!["-s", "-m", "5", "-X", "DELETE"];
            let auth_header;
            if let Some(key) = &self.api_key {
                auth_header = format!("Authorization: Bearer {key}");
                args.push("-H");
                args.push(&auth_header);
            }
            args.push(&url);
            let output = capture_command("curl", &args)
                .map_err(|e| format!("Failed to delete headscale node: {e}"))?;
            if !output.success {
                return Err(format!("curl error: {}", output.stderr));
            }
            Ok(())
        } else {
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
    }

    /// Observe node liveness by querying member list.
    pub fn observe_node_liveness(&self, node_name: &str) -> Result<bool, String> {
        let json_data = self.list_nodes()?;
        let nodes = parse_nodes_json(&json_data)?;
=======
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
>>>>>>> github/main

        for node in nodes {
            if node.name == node_name {
                return Ok(node.online);
            }
        }

        Err(format!("Node {} not found", node_name))
    }
}
<<<<<<< HEAD

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

        let nodes = parse_nodes_json(&raw)
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
        let binding = cynic_network::admit_mesh_provider(&adapter.provider())
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

    #[test]
    fn parses_rest_api_wrapped_nodes_json() {
        let raw = r#"{
            "nodes": [
                {
                    "id": "1",
                    "name": "builder",
                    "ipAddresses": ["100.64.0.1"],
                    "online": true
                },
                {
                    "id": 2,
                    "name": "cortex",
                    "ipAddresses": ["100.64.0.2"],
                    "online": false
                }
            ]
        }"#;
        let nodes = parse_nodes_json(raw).expect("parse rest nodes");
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].id, 1);
        assert_eq!(nodes[0].name, "builder");
        assert_eq!(nodes[0].ip_addresses, vec!["100.64.0.1"]);
        assert!(nodes[0].online);
        assert_eq!(nodes[1].id, 2);
        assert!(!nodes[1].online);
    }

    #[test]
    fn parses_cli_array_nodes_json() {
        let raw = r#"[
            {
                "id": 1,
                "name": "builder",
                "ip_addresses": ["100.64.0.1"],
                "online": true
            }
        ]"#;
        let nodes = parse_nodes_json(raw).expect("parse cli nodes");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, "builder");
        assert_eq!(nodes[0].ip_addresses, vec!["100.64.0.1"]);
    }

    #[test]
    fn parses_preauthkey_from_wrapped_or_raw() {
        let rest_json = r#"{"preAuthKey": {"key": "test-key-123"}}"#;
        assert_eq!(parse_preauthkey_response(rest_json), "test-key-123");

        let raw = "test-raw-key-456\n";
        assert_eq!(parse_preauthkey_response(raw), "test-raw-key-456");
    }

    #[test]
    fn parses_health_check_response() {
        let healthy_json = r#"{"status":"pass"}"#;
        assert!(healthy_json.contains("\"status\":\"pass\""));
    }
}
=======
>>>>>>> github/main
