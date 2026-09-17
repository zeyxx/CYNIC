//! Network observation domain (4-layer: physical / volume / usage / pressure).
//!
//! Pure parsers live in `probe`; the observe port is in `port`; canonical
//! types are re-exported from `cynic_protocol::network` so host-domain code
//! has a stable path (`crate::domain::network::*`) without duplicating policy.
pub mod actuator;
pub mod discovery;
pub mod dns;
pub mod firewall_policy;
pub mod freebox;
pub mod mesh_plan;
pub mod port;
pub mod probe;
pub mod registry_maintenance;
pub mod telemetry;
pub mod types;
pub mod watchdog_alerter;

pub use actuator::*;
pub use discovery::*;
pub use dns::*;
pub use firewall_policy::*;
pub use freebox::FreeboxProbePort;
pub use mesh_plan::*;
pub use registry_maintenance::*;
pub use telemetry::*;
pub use types::*;

pub use port::NetworkObservationPort;
pub use probe::{
    parse_addr_from_json, parse_dropped_packets_from_dev, parse_ifname_from_dev,
    parse_listening_ports_from_proc_tcp, parse_pressure_from_dev, parse_route_from_json,
    parse_usage_from_dev, parse_wireguard_telemetry,
};
pub use watchdog_alerter::reconcile_network_watchdog_outbox;
