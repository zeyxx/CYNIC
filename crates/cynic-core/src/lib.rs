//! CYNIC Core — pure domain types and ports.
//!
//! Zero-I/O: no `std::process`, no `reqwest`, no `tokio::spawn`.
//! This crate is the leaf of the DAG — nothing depends on it except higher layers.

pub mod domain;
pub mod fleet_mesh;
