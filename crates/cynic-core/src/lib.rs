//! CYNIC Core — pure domain entities and ports.
//!
//! Zero-I/O: no `std::process`, no `reqwest`, no `tokio`.
//! This crate is a leaf of the DAG — it depends only on `cynic-protocol`.
//!
//! SCOPE. `cynic-protocol` holds constitutional law (types, invariants, system
//! ports). `cynic-core` holds the CYNIC domain model: business entities,
//! domain invariants, and domain ports. The test: is the type constitutional
//! (a law of the system) or is it business (a CYNIC domain concept)?
//! Constitutional -> `cynic-protocol`. Business -> here.

pub mod fleet_mesh;
