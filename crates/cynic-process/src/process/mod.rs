//! Process-level observation domain (crash-loop detection).
//!
//! Pure logic lives in `probe`; canonical types come from
//! `cynic_protocol::health_signal` and this module's `types.rs`.
//! Adapters probe the substrate (systemd/Journald, OpenRC, etc.) and hand
//! observations in — no I/O here.

pub mod probe;
pub mod types;
pub use types::*;
