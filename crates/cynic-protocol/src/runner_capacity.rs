//! Runner capacity snapshots and usability decisions.
//!
//! WHY: placement must read supervised state, never probe cold. The snapshot
//! is the fact a runner publishes; the decide fn turns it into usable,
//! degraded, or muted. Unmeasured is muted, never zero-assumed.
//!
//! HOW: pure logic in this module (no I/O). Callers supply the snapshot,
//! the current time, the class floor, and the staleness window.
//!
//! NEGATIVE BOUNDARY: does not read or write files, does not perform network
//! I/O, does not spawn processes, never admits or schedules anything.

use serde::{Deserialize, Serialize};

/// Schema identifier for runner capacity snapshots.
pub const RUNNER_CAPACITY_SNAPSHOT_SCHEMA: &str = "cynic.fleet.capacity_snapshot.v1";
/// Window after which a snapshot reads as stale (mirrors the contract).
pub const RUNNER_STALENESS_SECS: u64 = 600;

/// One observed fact about a runner, published by supervision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunnerCapacitySnapshot {
    pub runner_id: String,
    pub observed_unix_s: u64,
    pub free_ram_bytes: Option<u64>,
    pub total_ram_bytes: Option<u64>,
    pub disk_avail_bytes: Option<u64>,
    pub state_root: Option<String>,
}

/// Placement usability of a runner.
#[derive(Debug, Clone, PartialEq)]
pub enum RunnerUsability {
    Usable,
    Degraded(String),
    Muted(String),
}

/// Decide usability of a runner snapshot.
///
/// - stale (older than `staleness_secs`) -> Muted, never trusted.
/// - any unmeasured field -> Muted, never zero-assumed.
/// - free RAM below `floor_ram_bytes` -> Degraded.
/// - otherwise -> Usable.
pub fn decide_runner_usability(
    snapshot: &RunnerCapacitySnapshot,
    now_unix_s: u64,
    floor_ram_bytes: u64,
    staleness_secs: u64,
) -> RunnerUsability {
    let age = now_unix_s.saturating_sub(snapshot.observed_unix_s);
    if age > staleness_secs {
        return RunnerUsability::Muted(format!(
            "stale snapshot: age {age}s exceeds window {staleness_secs}s"
        ));
    }
    let free = match snapshot.free_ram_bytes {
        Some(f) => f,
        None => {
            return RunnerUsability::Muted(
                "unmeasured free RAM: muted, never zero-assumed".to_string(),
            );
        }
    };
    match snapshot.disk_avail_bytes {
        Some(_) => {}
        None => {
            return RunnerUsability::Muted(
                "unmeasured disk availability: muted, never zero-assumed".to_string(),
            );
        }
    }
    match &snapshot.state_root {
        Some(_) => {}
        None => {
            return RunnerUsability::Muted(
                "unreported state root: muted, runner cannot receive work".to_string(),
            );
        }
    }
    if free < floor_ram_bytes {
        return RunnerUsability::Degraded(format!(
            "below floor: free {free} < floor {floor_ram_bytes}"
        ));
    }
    RunnerUsability::Usable
}

#[cfg(test)]
mod runner_capacity_tests {
    use super::*;

    const FLOOR: u64 = 2147483648;
    const STALENESS: u64 = 600;
    const NOW: u64 = 1_000_000;

    fn healthy(runner: &str) -> RunnerCapacitySnapshot {
        RunnerCapacitySnapshot {
            runner_id: runner.to_string(),
            observed_unix_s: NOW,
            free_ram_bytes: Some(FLOOR * 2),
            total_ram_bytes: Some(FLOOR * 4),
            disk_avail_bytes: Some(10_000_000_000),
            state_root: Some("/srv/cynic/runs".to_string()),
        }
    }

    #[test]
    fn fresh_above_floor_is_usable() {
        assert_eq!(
            decide_runner_usability(&healthy("builder"), NOW, FLOOR, STALENESS),
            RunnerUsability::Usable
        );
    }

    #[test]
    fn snapshot_missing_disk_is_muted() {
        let mut snap = healthy("builder");
        snap.disk_avail_bytes = None;
        assert!(matches!(
            decide_runner_usability(&snap, NOW, FLOOR, STALENESS),
            RunnerUsability::Muted(_)
        ));
    }

    #[test]
    fn stale_snapshot_is_muted() {
        let mut snap = healthy("builder");
        snap.observed_unix_s = NOW - STALENESS - 1;
        assert!(matches!(
            decide_runner_usability(&snap, NOW, FLOOR, STALENESS),
            RunnerUsability::Muted(_)
        ));
    }

    #[test]
    fn below_floor_is_degraded() {
        let mut snap = healthy("builder");
        snap.free_ram_bytes = Some(FLOOR - 1);
        assert!(matches!(
            decide_runner_usability(&snap, NOW, FLOOR, STALENESS),
            RunnerUsability::Degraded(_)
        ));
    }
}
