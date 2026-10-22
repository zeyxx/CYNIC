//! Fleet supervision: observe runner capacity on a schedule (read-only).
//!
//! WHY: placement must read supervised state, never probe cold. This arm
//! probes every registry runner (RAM, disk, state root) and writes the
//! snapshots to `infra/fleet-capacity.json`. Detection is automatic and
//! read-only; admission stays signed elsewhere.
//!
//! HOW: runners come from `list_runner_nodes` (dynamic registry, no
//! hardcoded names). Probes reuse `SshBuildRunner` (observation only, never
//! mutation). The registry write replaces one snapshot per runner.
//!
//! NEGATIVE BOUNDARY: never mutates runners, never admits or schedules
//! anything, never touches the network beyond the bounded probe argv.

use clap::Subcommand;
use serde::Serialize;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::runner_capacity::{
    RunnerCapacitySnapshot, RunnerUsability, decide_runner_usability,
};

use crate::{emit_json, fail, rust_source};

/// Minimum task class floor used to judge usability at observe time.
/// Placement applies its own per-class floor; observe reports against the
/// lightest known floor so degradation is visible early.
pub const OBSERVE_FLOOR_RAM_BYTES: u64 = 2147483648;
/// Window after which a snapshot reads as stale (mirrors the contract).
pub const OBSERVE_STALENESS_SECS: u64 = 600;

#[derive(Subcommand, Debug)]
pub enum FleetCommand {
    /// Observe every registry runner and record capacity snapshots (read-only
    /// on the runners; rewrites the local capacity registry).
    Observe {
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
}

pub fn execute_fleet(cmd: &FleetCommand) -> ExitCode {
    match cmd {
        FleetCommand::Observe { json } => execute_observe(*json),
    }
}

#[derive(Debug, Serialize)]
struct ObserveCliReport {
    schema: String,
    snapshots: Vec<RunnerCapacitySnapshot>,
    degraded: Vec<String>,
    muted: Vec<String>,
}

/// Replace the snapshot for one runner id, preserving the others.
pub fn upsert_snapshot(registry: &mut serde_json::Value, snap: &RunnerCapacitySnapshot) {
    let empty = serde_json::Value::Array(Vec::new());
    let snapshots = registry
        .as_object_mut()
        .and_then(|o| o.get_mut("snapshots"))
        .unwrap_or(&mut serde_json::Value::Null);
    if !snapshots.is_array() {
        *snapshots = empty;
    }
    let arr = snapshots.as_array_mut().expect("snapshots is an array");
    arr.retain(|s| s.get("runner_id").and_then(|v| v.as_str()) != Some(snap.runner_id.as_str()));
    arr.push(serde_json::json!({
        "runner_id": snap.runner_id,
        "observed_unix_s": snap.observed_unix_s,
        "free_ram_bytes": snap.free_ram_bytes,
        "disk_avail_bytes": snap.disk_avail_bytes,
        "state_root": snap.state_root,
    }));
}

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Read-only observe: probe every registry runner, record snapshots.
///
/// Never mutates runners. Rewrites `infra/fleet-capacity.json` with fresh
/// snapshots. Non-zero exit when any runner is muted or degraded so a cron
/// can alert.
fn execute_observe(json: bool) -> ExitCode {
    let candidates = cynic_host::registry::resolve::list_runner_nodes(None).unwrap_or_default();
    if candidates.is_empty() {
        return fail("no runner nodes in the registry".to_string());
    }

    let root = rust_source::workspace_root();
    let registry_path = root.join("infra/fleet-capacity.json");
    let registry_raw = match std::fs::read_to_string(&registry_path) {
        Ok(s) => s,
        Err(e) => return fail(format!("cannot read capacity registry: {e}")),
    };
    let mut registry: serde_json::Value = match serde_json::from_str(&registry_raw) {
        Ok(v) => v,
        Err(e) => return fail(format!("invalid JSON in capacity registry: {e}")),
    };

    let now = now_unix_s();
    let mut snapshots = Vec::new();
    let mut degraded = Vec::new();
    let mut muted = Vec::new();

    for entry in &candidates {
        let Some((user, probe_host)) = entry.ssh_target.split_once('@') else {
            continue;
        };
        let probe = cynic_node::cli_factory::ssh_build_runner(
            probe_host.to_string(),
            user.to_string(),
            String::new(),
            None,
            String::new(),
            None,
            None,
        );
        let snap = RunnerCapacitySnapshot {
            runner_id: entry.node_id.clone(),
            observed_unix_s: now,
            free_ram_bytes: probe.probe_free_ram_bytes(),
            disk_avail_bytes: probe.probe_disk_avail_bytes(),
            state_root: probe.probe_state_root(),
        };
        match decide_runner_usability(&snap, now, OBSERVE_FLOOR_RAM_BYTES, OBSERVE_STALENESS_SECS) {
            RunnerUsability::Usable => {}
            RunnerUsability::Degraded(_) => degraded.push(entry.node_id.clone()),
            RunnerUsability::Muted(_) => muted.push(entry.node_id.clone()),
        }
        upsert_snapshot(&mut registry, &snap);
        snapshots.push(snap);
    }

    let out = match serde_json::to_string_pretty(&registry) {
        Ok(s) => s,
        Err(e) => return fail(format!("cannot serialize capacity registry: {e}")),
    };
    if let Err(e) = std::fs::write(&registry_path, out) {
        return fail(format!("cannot write capacity registry: {e}"));
    }

    if json {
        let report = ObserveCliReport {
            schema: "cynic.cli.fleet_observe.v1".to_string(),
            snapshots,
            degraded: degraded.clone(),
            muted: muted.clone(),
        };
        emit_json(&report, !degraded.is_empty() || !muted.is_empty())
    } else {
        for s in &snapshots {
            let state = if muted.iter().any(|m| m == &s.runner_id) {
                "muted"
            } else if degraded.iter().any(|d| d == &s.runner_id) {
                "degraded"
            } else {
                "usable"
            };
            println!(
                "{}: {} (ram={:?}, disk={:?}, root={:?})",
                s.runner_id, state, s.free_ram_bytes, s.disk_avail_bytes, s.state_root
            );
        }
        if degraded.is_empty() && muted.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod fleet_observe_tests {
    use super::*;

    fn snap(runner: &str, observed: u64) -> RunnerCapacitySnapshot {
        RunnerCapacitySnapshot {
            runner_id: runner.to_string(),
            observed_unix_s: observed,
            free_ram_bytes: Some(1),
            disk_avail_bytes: Some(1),
            state_root: Some("/srv".to_string()),
        }
    }

    #[test]
    fn upsert_replaces_stale_snapshot_same_runner() {
        let mut registry = serde_json::json!({
            "schema": "cynic.fleet.capacity.v1",
            "snapshots": [
                {"runner_id": "builder", "observed_unix_s": 1,
                 "free_ram_bytes": 1, "disk_avail_bytes": 1,
                 "state_root": "/old"}
            ]
        });
        upsert_snapshot(&mut registry, &snap("builder", 2));
        let arr = registry
            .get("snapshots")
            .and_then(|v| v.as_array())
            .expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(
            arr[0].get("observed_unix_s").and_then(|v| v.as_u64()),
            Some(2)
        );
        assert_eq!(
            arr[0].get("state_root").and_then(|v| v.as_str()),
            Some("/srv")
        );
    }
}
