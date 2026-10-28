//! Execution port for lifecycle gates/repairs.
//!
//! Domain must not hard-wire `Command::new("cynic")`. CLI injects
//! [`LocalArgvRunner`]; tests inject fakes.
//!
//! The [`ArgvRunner`] trait and [`ArgvOutput`] type used to be defined here.
//! They now live in [`cynic_protocol::argv`], beside [`cynic_protocol::argv_allow`]
//! which already held the admission policy: the port that runs an argv must not
//! sit above its own policy. `cynic-vcs` carried a third copy with a narrower
//! contract (no `Serialize`, and its `MapArgvRunner` hidden behind `cfg(test)`).
//!
//! What stays here is the adapter layer: [`LocalArgvRunner`] spawns, and
//! [`MapArgvRunner`] fabricates for tests in other crates — both depend on
//! `cynic-build`, which an L0 port module must not.

use std::collections::HashMap;

pub use cynic_protocol::argv::{ArgvOutput, ArgvRunner};
pub use cynic_process::{LocalArgvRunner, MapArgvRunner};

/// Split a shell-like command string into whitespace tokens.
pub fn split_command(command: &str) -> Vec<&str> {
    command.split_whitespace().collect()
}

pub fn json_status(stdout: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    v.get("status")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

pub fn parse_json(stdout: &str) -> Option<serde_json::Value> {
    serde_json::from_str(stdout.trim()).ok()
}
