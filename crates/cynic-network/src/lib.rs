//! Network observation domain (4-layer: physical / volume / usage / pressure).
//!
//! Conforms to Functional Core / Imperative Shell (FCIS) hybrid architecture:
//! - Functional Core: pure data models (`types`), pure parsers (`probe`),
//!   pure drift detection (`discovery`), and pure planning (`firewall_policy`, `mesh_plan`).
//! - Imperative Shell: side-effecting I/O isolated under `adapters/`.

pub mod actuator;
pub mod adapters;
pub mod discovery;
pub mod dns;
pub mod dns_activation;
pub mod dns_resolver;
pub mod firewall_policy;
pub mod freebox;
pub mod mesh_bundle;
pub mod mesh_plan;
pub mod fleet_membership;
pub mod plane_vocabulary;
pub mod port;
pub mod probe;
pub mod quic;
pub mod registry_maintenance;
pub mod telemetry;
pub mod transport_readback;
pub mod transport_security;
pub mod types;
pub mod watchdog_alerter;

// Re-export adapters for direct and legacy access
pub use adapters::finance::NetworkFinanceMeter;
pub use adapters::headscale::{HeadscaleClient, HeadscaleConfig};
pub use adapters::host_probe::{LocalNetworkAdapter, LocalNetworkDiscoveryProbeAdapter};



// Backward compatibility module aliases
pub use adapters::finance as finance_adapters;
pub use adapters::headscale as headscale_client;

pub use actuator::*;
pub use discovery::*;
pub use dns::*;
pub use dns_resolver::{DnsEndpoint, DnsResolutionPort};
pub use firewall_policy::*;
pub use freebox::{
    FreeboxDmzConfig, FreeboxNatRule, FreeboxNatRuleSpec, FreeboxPort, FreeboxProbePort,
    FreeboxWanStatus, FREEBOX_DMZ_CONFIG_SCHEMA, FREEBOX_NAT_RULE_SCHEMA,
    FREEBOX_WAN_STATUS_SCHEMA,
};
pub use mesh_bundle::*;
pub use mesh_plan::*;
pub use registry_maintenance::*;
pub use telemetry::*;
pub use transport_readback::*;
pub use transport_security::*;
pub use types::*;

pub use port::NetworkObservationPort;
pub use probe::{
    parse_addr_from_json, parse_dropped_packets_from_dev, parse_ifname_from_dev,
    parse_listening_ports_from_proc_tcp, parse_pressure_from_dev, parse_route_from_json,
    parse_usage_from_dev, parse_wireguard_telemetry,
};
pub use watchdog_alerter::reconcile_network_watchdog_outbox;
