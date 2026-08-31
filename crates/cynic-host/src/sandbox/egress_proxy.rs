//! Capability-aware egress proxy.
//!
//! Routes external HTTP requests through a local proxy that validates
//! the X-CYNIC-Capability-Token header before forwarding.

use serde::{Deserialize, Serialize};

pub const EGRESS_PROXY_SCHEMA: &str = "cynic.sandbox.egress_proxy.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgressProxyConfig {
    pub schema: String,
    pub listen_addr: String,
    pub allowed_methods: Vec<String>,
    pub max_bytes_per_session: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgressProxyReport {
    pub schema: String,
    pub listening: bool,
    pub listen_addr: String,
    pub error: Option<String>,
}

pub trait EgressProxyPort {
    fn start(&self, config: &EgressProxyConfig) -> Result<EgressProxyReport, String>;
    fn stop(&self) -> Result<(), String>;
}

pub fn null_egress_proxy_port() -> NullEgressProxyPort {
    NullEgressProxyPort
}

pub struct NullEgressProxyPort;

impl EgressProxyPort for NullEgressProxyPort {
    fn start(&self, _config: &EgressProxyConfig) -> Result<EgressProxyReport, String> {
        Err("egress proxy not implemented for this platform".to_string())
    }
    fn stop(&self) -> Result<(), String> {
        Err("egress proxy not implemented for this platform".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn egress_config_methods() {
        let c = EgressProxyConfig {
            schema: EGRESS_PROXY_SCHEMA.to_string(),
            listen_addr: "127.0.0.1:18080".to_string(),
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            max_bytes_per_session: 10 * 1024 * 1024,
        };
        assert_eq!(c.allowed_methods.len(), 2);
    }
}
