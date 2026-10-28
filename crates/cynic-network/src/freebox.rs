//! Pure Freebox OS Domain Model and Ports
//! Follows Alistair Cockburn's Hexagonal Architecture (Ports and Adapters).
//! Core domain is 100% Zero-I/O: contains only data types, pure parsers,
//! and the abstract ports. Zero network sockets or HTTP clients.

use serde::{Deserialize, Serialize};

use crate::types::FreeboxLanObservation;

pub const FREEBOX_WAN_STATUS_SCHEMA: &str = "cynic.network.freebox_wan_status.v1";
pub const FREEBOX_DMZ_CONFIG_SCHEMA: &str = "cynic.network.freebox_dmz_config.v1";
pub const FREEBOX_NAT_RULE_SCHEMA: &str = "cynic.network.freebox_nat_rule.v1";

/// Live WAN connection status of the Freebox router.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FreeboxWanStatus {
    pub schema: String,
    pub state: String,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
    pub rate_down_bps: Option<u64>,
    pub rate_up_bps: Option<u64>,
    pub bytes_down_total: Option<u64>,
    pub bytes_up_total: Option<u64>,
}

/// DMZ configuration on the Freebox router.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FreeboxDmzConfig {
    pub schema: String,
    pub enabled: bool,
    pub ip: String,
}

/// A port redirection / NAT rule on the Freebox router.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FreeboxNatRule {
    pub schema: String,
    pub id: u32,
    pub enabled: bool,
    pub ip: String,
    pub protocol: String,
    pub wan_port_start: u16,
    pub wan_port_end: u16,
    pub lan_port: u16,
    pub comment: String,
}

/// Input specification to create or update a NAT rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FreeboxNatRuleSpec {
    pub enabled: bool,
    pub ip: String,
    pub protocol: String,
    pub wan_port_start: u16,
    pub wan_port_end: u16,
    pub lan_port: u16,
    pub comment: String,
}

/// Core port for Freebox OS router operations.
pub trait FreeboxPort: Send + Sync {
    /// Probe the LAN topology (unauthenticated or authenticated).
    fn probe_lan(&self, endpoint: &str) -> Result<FreeboxLanObservation, String>;

    /// Read WAN status (IPv4, IPv6, throughput).
    fn read_wan(&self, endpoint: &str) -> Result<FreeboxWanStatus, String>;

    /// Read DMZ configuration.
    fn read_dmz(&self, endpoint: &str) -> Result<FreeboxDmzConfig, String>;

    /// Update DMZ configuration (requires explicit operator mutation).
    fn write_dmz(
        &self,
        endpoint: &str,
        enabled: bool,
        ip: &str,
    ) -> Result<FreeboxDmzConfig, String>;

    /// List all NAT/port-forwarding rules.
    fn list_nat_rules(&self, endpoint: &str) -> Result<Vec<FreeboxNatRule>, String>;

    /// Add a NAT/port-forwarding rule (requires explicit operator mutation).
    fn add_nat_rule(
        &self,
        endpoint: &str,
        spec: &FreeboxNatRuleSpec,
    ) -> Result<FreeboxNatRule, String>;

    /// Delete a NAT/port-forwarding rule by id (requires explicit operator mutation).
    fn delete_nat_rule(&self, endpoint: &str, rule_id: u32) -> Result<(), String>;
}

/// Deprecated legacy trait alias to prevent breaking existing consumers.
pub trait FreeboxProbePort: Send + Sync {
    fn probe_freebox_lan(&self, endpoint: &str) -> Result<FreeboxLanObservation, String>;
}
