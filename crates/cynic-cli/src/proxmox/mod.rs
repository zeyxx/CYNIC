//! Proxmox supervision — CLI surface (thin facade).
//!
//! Domain types live in `cynic_protocol::proxmox` (pure types).
//! Substrate adapter lives in `cynic_node::adapters::proxmox_control`.
//! This module: clap parse → inject adapter → call node → print JSON/human.

pub mod cli;

pub use cli::{HypervisorCommand, execute_hypervisor};
