//! Network namespace adapter (ip netns / unshare --net).

use serde::{Deserialize, Serialize};

pub const NETNS_SCHEMA: &str = "cynic.sandbox.netns.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetnsConfig {
    pub schema: String,
    pub name: String,
    pub default_deny: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetnsReport {
    pub schema: String,
    pub name: String,
    pub created: bool,
    pub error: Option<String>,
}

pub trait NetnsPort {
    fn create(&self, config: &NetnsConfig) -> Result<NetnsReport, String>;
    fn destroy(&self, name: &str) -> Result<(), String>;
}

pub fn null_netns_port() -> NullNetnsPort {
    NullNetnsPort
}

pub struct NullNetnsPort;

impl NetnsPort for NullNetnsPort {
    fn create(&self, _config: &NetnsConfig) -> Result<NetnsReport, String> {
        Err("netns adapter not implemented for this platform".to_string())
    }
    fn destroy(&self, _name: &str) -> Result<(), String> {
        Err("netns adapter not implemented for this platform".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netns_config_default_deny() {
        let c = NetnsConfig {
            schema: NETNS_SCHEMA.to_string(),
            name: "agent-test".to_string(),
            default_deny: true,
        };
        assert!(c.default_deny);
    }
}
