//! Sensory organ readers — the organism perceiving external data stores.
//!
//! Distinct from organ/ (InferenceOrgan: Dog backend registry).
//! Each sense reads an external data store in read-only mode.

pub mod hermes_x;
pub mod rtk;
pub mod tailscale;

use crate::domain::organ::OrganPort;
use std::sync::Arc;

/// Build the sense registry at startup. Best-effort: missing organs are skipped.
pub fn build_sense_registry(project_root: &str) -> Vec<Arc<dyn OrganPort>> {
    let mut senses: Vec<Arc<dyn OrganPort>> = Vec::new();

    // RTK — token metabolism (SQLite)
    let rtk_db = dirs::data_local_dir()
        .unwrap_or_default()
        .join("rtk/history.db");
    if rtk_db.exists() {
        senses.push(Arc::new(rtk::RtkReader::new(
            rtk_db,
            project_root.to_string(),
        )));
    }

    // Tailscale — fleet nervous system (local CLI)
    // Fleet nodes derived from fleet.toml: only track machines with status = "active".
    // Nodes with status = "offline" are expected-down — no alerts, no probing.
    if which::which("tailscale").is_ok() {
        let fleet_nodes = parse_fleet_active_nodes();
        if !fleet_nodes.is_empty() {
            senses.push(Arc::new(tailscale::TailscaleReader::new(fleet_nodes)));
        }
    }

    // Hermes X — social perception (JSONL + filesystem)
    let hermes_x_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".cynic/organs/hermes/x");
    if hermes_x_dir.exists() {
        senses.push(Arc::new(hermes_x::HermesXReader::new(hermes_x_dir)));
    }

    senses
}

/// Parse `~/.config/cynic/fleet.toml` and return hostnames of machines with `status = "active"`.
/// Falls back to empty vec if fleet.toml is missing or unparseable — no fleet tracking.
fn parse_fleet_active_nodes() -> Vec<String> {
    let path = dirs::config_dir()
        .unwrap_or_default()
        .join("cynic/fleet.toml");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(table) = content.parse::<toml::Table>() else {
        tracing::warn!("fleet.toml parse error");
        return Vec::new();
    };
    let Some(machines) = table.get("machine").and_then(|m| m.as_table()) else {
        return Vec::new();
    };
    machines
        .iter()
        .filter_map(|(name, val)| {
            let t = val.as_table()?;
            let status = t.get("status").and_then(|s| s.as_str()).unwrap_or("active");
            if status == "active" {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect()
}
