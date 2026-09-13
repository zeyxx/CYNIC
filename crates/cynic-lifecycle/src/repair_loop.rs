//! Bounded observe → repair → read-back loop.

use super::observe::ops_report;
use super::port::ArgvRunner;
use super::repair::{repair_apply, repair_plan};
use super::time::now_ms;
use super::types::RepairReport;
use serde::Serialize;
use std::path::Path;

pub const REPAIR_LOOP_SCHEMA: &str = "cynic.update_lifecycle.repair_loop.v1";

#[derive(Debug, Clone, Serialize)]
pub struct RepairLoopCycle {
    pub index: u32,
    pub repair: RepairReport,
    pub readback_status: &'static str,
    pub remaining_findings: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct RepairLoopReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_cycles: u32,
    pub cycles: Vec<RepairLoopCycle>,
    pub stop_reason: &'static str,
    pub generated_at_unix_ms: u64,
}

pub fn repair_loop(
    state_path: &Path,
    cycles: u32,
    yes: bool,
    runner: &dyn ArgvRunner,
) -> RepairLoopReport {
    if cycles == 0 {
        return report("blocked", false, 0, Vec::new(), "cycles_must_be_positive");
    }
    if cycles > 1 && !yes {
        return report(
            "blocked",
            false,
            cycles,
            Vec::new(),
            "repeat_apply_requires_yes",
        );
    }
    let mut evidence = Vec::new();
    let mut mutates = false;
    let mut status = "ok";
    let mut stop_reason = "cycle_limit_reached";
    for index in 1..=cycles {
        let repair = if yes {
            repair_apply(state_path, true, runner)
        } else {
            repair_plan(state_path, runner)
        };
        mutates |= repair.mutates;
        let readback = ops_report(state_path, runner);
        let remaining = readback.findings.len();
        let readback_status = readback.status;
        if repair.status == "degraded" || readback.status == "degraded" {
            status = "degraded";
            stop_reason = "repair_or_readback_failed";
        } else if remaining == 0 {
            stop_reason = "converged_no_findings";
        }
        evidence.push(RepairLoopCycle {
            index,
            repair,
            readback_status,
            remaining_findings: remaining,
        });
        if status == "degraded" || remaining == 0 || !yes {
            break;
        }
    }
    report(status, mutates, cycles, evidence, stop_reason)
}
fn report(
    status: &'static str,
    mutates: bool,
    requested_cycles: u32,
    cycles: Vec<RepairLoopCycle>,
    stop_reason: &'static str,
) -> RepairLoopReport {
    RepairLoopReport {
        schema: REPAIR_LOOP_SCHEMA,
        status,
        mutates,
        requested_cycles,
        cycles,
        stop_reason,
        generated_at_unix_ms: now_ms(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::port::MapArgvRunner;
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn rejects_unbounded_or_implicit_repeat() {
        let path = PathBuf::from("/tmp/cynic-repair-loop-test-state.json");
        let runner = MapArgvRunner::default();
        assert_eq!(
            repair_loop(&path, 0, false, &runner).stop_reason,
            "cycles_must_be_positive"
        );
        assert_eq!(
            repair_loop(&path, 2, false, &runner).stop_reason,
            "repeat_apply_requires_yes"
        );
    }
}
