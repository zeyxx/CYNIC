//! Network observation domain (4-layer: physical / volume / usage / pressure).
//!
//! Conforms to Functional Core / Imperative Shell (FCIS) hybrid architecture:
//! - Functional Core: pure data models (`types`), pure parsers (`probe`),
//!   pure drift detection (`discovery`), and pure planning (`firewall_policy`, `mesh_plan`).
//! - Imperative Shell: side-effecting I/O isolated under `adapters/`.

pub mod actuator;
pub mod adapters;
pub mod ports;
pub mod topology;
pub mod command_surface;
pub mod discovery;
pub mod dns;
pub mod endpoint_observation;
pub use endpoint_observation::*;
pub use dns as dns_resolver;
pub use dns::activation as dns_activation;
pub mod firewall_policy;
pub mod freebox;
pub mod peer_handshake;
pub use peer_handshake::*;
pub mod peer_validator;
pub use peer_validator::*;
pub mod mesh;
pub mod mesh_bundle;
pub mod notification_outbox;
pub use notification_outbox::*;
pub mod observability;
pub use observability::*;
pub mod mesh_plan;
pub mod mesh_registry;
pub mod fleet_membership;
pub mod plane_vocabulary;
pub mod port;
pub mod probe;
pub mod preflight;
pub mod quic;
pub mod registry_maintenance;
pub mod telemetry;
pub mod transport;
pub mod transport_readback;
pub mod transport_security;
pub mod types;
pub mod usecases;
pub mod kernel_tunnel;
pub mod watchdog_alerter;
pub mod wireguard_readback;

// Re-export adapters for direct and legacy access
pub use adapters::finance::NetworkFinanceMeter;
pub use adapters::headscale::{HeadscaleClient, HeadscaleConfig};
pub use adapters::host_probe::{LocalNetworkAdapter, LocalNetworkDiscoveryProbeAdapter};
pub use adapters::workspace_registry::endpoints_from_workspace_registry;



// Backward compatibility module aliases
pub use adapters::finance as finance_adapters;
pub use adapters::headscale as headscale_client;

pub use actuator::*;
pub use command_surface::*;
pub use discovery::*;
pub use topology::*;
pub use dns::*;
pub use dns_resolver::{DnsEndpoint, DnsResolutionPort};
pub use firewall_policy::*;
pub use freebox::{
    FreeboxDmzConfig, FreeboxNatRule, FreeboxNatRuleSpec, FreeboxPort, FreeboxProbePort,
    FreeboxWanStatus, FREEBOX_DMZ_CONFIG_SCHEMA, FREEBOX_NAT_RULE_SCHEMA,
    FREEBOX_WAN_STATUS_SCHEMA,
};
pub use mesh::*;
pub use mesh_bundle::*;
pub use mesh_plan::*;
pub use mesh_registry::*;
pub use registry_maintenance::*;
pub use telemetry::*;
pub use transport_readback::*;
pub use transport_security::*;
pub use types::*;

pub use ports::*;
pub use ports::mesh_coordination::*;
pub use probe::{
    parse_addr_from_json, parse_dropped_packets_from_dev, parse_ifname_from_dev,
    parse_listening_ports_from_proc_tcp, parse_pressure_from_dev, parse_route_from_json,
    parse_usage_from_dev, parse_wireguard_telemetry,
};
pub use watchdog_alerter::reconcile_network_watchdog_outbox;
pub use kernel_tunnel::*;
pub use wireguard_readback::*;
