//! Agent isolation profiles and sandbox configuration.
//!
//! Declares the isolation contract for agent execution contexts.
//! Runtime surfaces select the concrete sandbox strategy based on this profile.

use serde::{Deserialize, Serialize};

pub const AGENT_ISOLATION_PROFILE_SCHEMA: &str = "cynic.agent_isolation_profile.v1";
pub const CAPABILITY_EGRESS_ROUTE_SCHEMA: &str = "cynic.capability_egress_route.v1";

/// Sandbox execution strategy for an agent session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxStrategy {
    /// Bubblewrap rootless (Linux namespaces, no root required).
    /// For ephemeral local agents (CLI, OMP, Claude Code, Codex).
    BwrapEphemeral,
    /// LXC unprivileged container (Proxmox VE).
    /// For long-running cluster workers (builds, Askesis).
    LxcUnprivileged,
}

/// Network isolation configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkIsolationProfile {
    /// No outbound network. Only loopback + UDS bind.
    /// For pure local inference agents.
    ZeroEgress,
    /// Outbound via capability-aware proxy sidecar.
    /// For agents needing external APIs (web search, git, Solana RPC).
    ProxyEgress,
}

/// Declares the isolation contract for an agent session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentIsolationProfile {
    pub schema: String,
    pub sandbox: SandboxStrategy,
    pub network: NetworkIsolationProfile,
    /// Path to the UDS socket for inference runtime communication.
    pub inference_uds_path: String,
    /// Rootfs mount mode.
    pub rootfs_readonly: bool,
    /// Workspace tmpfs size limit in bytes (0 = unlimited).
    pub tmpfs_limit_bytes: u64,
}

impl AgentIsolationProfile {
    pub fn ephemeral_local(uds_path: &str) -> Self {
        Self {
            schema: AGENT_ISOLATION_PROFILE_SCHEMA.to_string(),
            sandbox: SandboxStrategy::BwrapEphemeral,
            network: NetworkIsolationProfile::ZeroEgress,
            inference_uds_path: uds_path.to_string(),
            rootfs_readonly: true,
            tmpfs_limit_bytes: 0,
        }
    }

    pub fn long_duration_cluster(uds_path: &str) -> Self {
        Self {
            schema: AGENT_ISOLATION_PROFILE_SCHEMA.to_string(),
            sandbox: SandboxStrategy::LxcUnprivileged,
            network: NetworkIsolationProfile::ProxyEgress,
            inference_uds_path: uds_path.to_string(),
            rootfs_readonly: true,
            tmpfs_limit_bytes: 0,
        }
    }
}

/// Capability-aware egress route for external API calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityEgressRoute {
    pub schema: String,
    /// Allowed RPC methods (e.g., "getLatestBlockhash", "simulateTransaction").
    pub allowed_methods: Vec<String>,
    /// Max requests per second.
    pub rate_limit_rps: u32,
    /// Total bytes allowed per session.
    pub max_bytes_per_session: u64,
}

impl CapabilityEgressRoute {
    pub fn solana_rpc() -> Self {
        Self {
            schema: CAPABILITY_EGRESS_ROUTE_SCHEMA.to_string(),
            allowed_methods: vec![
                "getLatestBlockhash".to_string(),
                "simulateTransaction".to_string(),
                "sendTransaction".to_string(),
            ],
            rate_limit_rps: 10,
            max_bytes_per_session: 10 * 1024 * 1024, // 10 MB
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_profile_has_zero_egress() {
        let p = AgentIsolationProfile::ephemeral_local("/tmp/test.sock");
        assert_eq!(p.sandbox, SandboxStrategy::BwrapEphemeral);
        assert_eq!(p.network, NetworkIsolationProfile::ZeroEgress);
        assert!(p.rootfs_readonly);
    }

    #[test]
    fn cluster_profile_has_proxy_egress() {
        let p = AgentIsolationProfile::long_duration_cluster("/tmp/test.sock");
        assert_eq!(p.sandbox, SandboxStrategy::LxcUnprivileged);
        assert_eq!(p.network, NetworkIsolationProfile::ProxyEgress);
    }

    #[test]
    fn solana_egress_route_has_expected_methods() {
        let r = CapabilityEgressRoute::solana_rpc();
        assert!(
            r.allowed_methods
                .contains(&"simulateTransaction".to_string())
        );
        assert!(r.allowed_methods.contains(&"sendTransaction".to_string()));
    }
}
