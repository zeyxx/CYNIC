pub mod audit;
pub mod model;
pub mod pins;
pub mod plan;
pub mod probe;
pub mod render;
pub mod report;
pub mod specs;
pub mod upgrade;

use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cli_json::emit_pretty_json;
use crate::dependency::audit::build_deps_audit;
use crate::dependency::pins::load_pins;
use crate::dependency::plan::build_plan;
use crate::dependency::render::{print_deps_human, print_plan_human};
use crate::dependency::report::build_deps_report;
use crate::dependency::upgrade::execute_deps_upgrade as execute_deps_upgrade_impl;

/// `cynic dependency` — human by default, `--json` for agents.
pub(crate) fn execute_deps(json: bool) -> ExitCode {
    let (pins_path, pins) = load_pins();
    let report = build_deps_report(&pins_path, pins, now_ms());
    if json {
        return emit_pretty_json(&report);
    }
    print_deps_human(&report);
    ExitCode::SUCCESS
}

/// `cynic dependency plan` — human by default.
pub(crate) fn execute_deps_plan(json: bool) -> ExitCode {
    let (pins_path, pins) = load_pins();
    let deps = build_deps_report(&pins_path, pins, now_ms());
    let plan = build_plan(&deps, now_ms());
    if json {
        return emit_pretty_json(&plan);
    }
    print_plan_human(&plan);
    ExitCode::SUCCESS
}

/// `cynic dependency audit` — prioritized normalization cockpit.
pub(crate) fn execute_deps_audit(json: bool) -> ExitCode {
    let (pins_path, pins) = load_pins();
    let deps = build_deps_report(&pins_path, pins, now_ms());
    let audit = build_deps_audit(&deps);
    if json {
        return emit_pretty_json(&audit);
    }
    crate::dependency::render::print_audit_human(&audit);
    ExitCode::SUCCESS
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub(crate) fn execute_deps_upgrade(
    json: bool,
    execute_with_apply: bool,
    yes: bool,
    confirm: Option<&str>,
    only: &[String],
    source: Option<&str>,
) -> ExitCode {
    execute_deps_upgrade_impl(
        json,
        execute_with_apply,
        yes,
        confirm,
        only,
        source,
        now_ms(),
        emit_pretty_json,
    )
}
