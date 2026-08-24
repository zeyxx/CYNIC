//! Network observation domain (4-layer: physical / volume / usage / pressure).
//!
//! Pure parsers live in `probe`; the observe port is in `port`; canonical
//! types are re-exported from `cynic_protocol::network` so host-domain code
//! has a stable path (`crate::domain::network::*`) without duplicating policy.
mod port;
mod probe;
pub mod types;

pub use port::NetworkObservationPort;
pub use types::*;
pub use probe::{
    parse_addr_from_json, parse_ifname_from_dev, parse_pressure_from_dev, parse_route_from_json,
    parse_usage_from_dev,
};
