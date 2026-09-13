//! Pure Freebox OS Domain Model and Ports
//! Follows Alistair Cockburn's Hexagonal Architecture (Ports and Adapters).
//! Core domain is 100% Zero-I/O: contains only data types, pure parsers,
//! and the abstract `FreeboxProbePort` trait. Zero network sockets or HTTP clients.

use cynic_protocol::{FreeboxHostEntry, FreeboxLanObservation};

pub trait FreeboxProbePort: Send + Sync {
    fn probe_freebox_lan(&self, endpoint: &str) -> Result<FreeboxLanObservation, String>;
}
