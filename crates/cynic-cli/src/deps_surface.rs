//! Thin CLI surface for ecosystem tech-watch.
//!
//! Business logic lives in `cynic_host::domain::ecosystem_package`:
//! types / layout / port / registry / providers (not a monolith).
//!
//! This module: probe pins, print human/JSON, consent (`--yes` / token).
//! Pins SSOT: `infra/ecosystem-pins.json` (override: `CYNIC_ECOSYSTEM_PINS`).

use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cli_json::emit_pretty_json;
use crate::deps_surface_audit::build_deps_audit;
use crate::deps_surface_pins::load_pins;
use crate::deps_surface_plan::build_plan;
use crate::deps_surface_render::{print_deps_human, print_plan_human};
use crate::deps_surface_report::build_deps_report;
use crate::deps_surface_upgrade::execute_deps_upgrade as execute_deps_upgrade_impl;

// ── Public entrypoints ──────────────────────────────────────────────────────

/// `cynic deps` — human by default, `--json` for agents.
pub(crate) fn execute_deps(json: bool) -> ExitCode {
    let (pins_path, pins) = load_pins();
    let report = build_deps_report(&pins_path, pins, now_ms());
    if json {
        return emit_pretty_json(&report);
    }
    print_deps_human(&report);
    ExitCode::SUCCESS
}

/// `cynic deps plan` — human by default.
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

/// `cynic deps audit` — prioritized normalization cockpit.
pub(crate) fn execute_deps_audit(json: bool) -> ExitCode {
    let (pins_path, pins) = load_pins();
    let deps = build_deps_report(&pins_path, pins, now_ms());
    let audit = build_deps_audit(&deps);
    if json {
        return emit_pretty_json(&audit);
    }
    crate::deps_surface_render::print_audit_human(&audit);
    ExitCode::SUCCESS
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// One-click / human-validated upgrade.
///
/// - default: show plan (human)
/// - `--yes`: apply all scripted candidates in one step (explicit human consent)
/// - `--apply --confirm TOKEN`: non-interactive (agents/CI)
/// - `--json`: machine report
pub(crate) fn execute_deps_upgrade(
    json: bool,
    execute_with_apply: bool,
    yes: bool,
    confirm: Option<&str>,
    only: &[String],
) -> ExitCode {
    execute_deps_upgrade_impl(
        json,
        execute_with_apply,
        yes,
        confirm,
        only,
        now_ms(),
        emit_pretty_json,
    )
}
