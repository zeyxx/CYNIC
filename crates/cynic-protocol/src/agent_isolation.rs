use serde::{Deserialize, Serialize};

pub const AGENT_ISOLATION_PROFILE_SCHEMA: &str = "cynic.agent_isolation_profile.v1";
pub const CAPABILITY_EGRESS_ROUTE_SCHEMA: &str = "cynic.capability_egress_route.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxStrategy {
    BwrapEphemeral,
    LxcUnprivileged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkIsolationProfile {
    ZeroEgress,
    ProxyEgress,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentIsolationProfile {
    pub schema: String,
    pub sandbox: SandboxStrategy,
    pub network: NetworkIsolationProfile,
    pub inference_uds_path: String,
    pub rootfs_readonly: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityEgressRoute {
    pub schema: String,
    pub allowed_methods: Vec<String>,
    pub rate_limit_rps: u32,
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
            max_bytes_per_session: 10 * 1024 * 1024,
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
