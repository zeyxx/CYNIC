//! Repair plan/apply from ops findings (phase-filtered).
//!
//! Execution only through [`ArgvRunner`] — no domain-owned `Command::new("cynic")`.

use std::path::Path;

use super::observe::ops_report;
use super::policy::phase_policy;
use super::port::{ArgvRunner, split_command};
use super::state_store::{load_state, save_state};
use super::time::now_ms;
use super::types::{
    MutationClass, RepairAction, RepairReport, RepairResult, UPDATE_LIFECYCLE_REPAIR_SCHEMA,
};

/// Plan repairs for open findings allowed by the current phase.
pub fn repair_plan(state_path: &Path, runner: &dyn ArgvRunner) -> RepairReport {
    let state = load_state(state_path);
    let policy = phase_policy(&state.current_phase);
    let ops = ops_report(state_path, runner);
    let mut actions: Vec<RepairAction> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    for f in &ops.findings {
        if !f.phase_allows_repair {
            continue;
        }
        let Some(cmd) = &f.repair_command else {
            continue;
        };
        let id = f
            .repair_id
            .clone()
            .unwrap_or_else(|| format!("repair_{}", f.id));
        if !seen.insert(id.clone()) {
            if let Some(a) = actions.iter_mut().find(|a| a.id == id) {
                a.finding_ids.push(f.id.clone());
            }
            continue;
        }

        let mutation_class = classify_command(cmd);
        if mutation_class != MutationClass::ObserveOnly
            && !policy.allowed_mutations.contains(&mutation_class)
        {
            continue;
        }

        actions.push(RepairAction {
            id,
            command: cmd.clone(),
            mutates: command_mutates(cmd),
            mutation_class,
            finding_ids: vec![f.id.clone()],
            purpose: f.summary.clone(),
        });
    }

    if state.last_gate_ok != Some(true) && !actions.iter().any(|a| a.id == "run_gate") {
        actions.insert(
            0,
            RepairAction {
                id: "run_gate".into(),
                command: "cynic lifecycle gate --yes".into(),
                mutates: true, // writes lifecycle state
                mutation_class: MutationClass::ObserveOnly,
                finding_ids: vec![],
                purpose: "establish phase gate evidence".into(),
            },
        );
    }

    let n_actions = actions.len();
    RepairReport {
        schema: UPDATE_LIFECYCLE_REPAIR_SCHEMA,
        status: if n_actions == 0 { "ok" } else { "planned" },
        phase: state.current_phase.as_str(),
        mutates: false,
        actions,
        results: vec![],
        message: if ops.findings.is_empty() {
            "no findings — organism observation surfaces quiet".into()
        } else {
            format!(
                "{} finding(s), {} repair action(s) planned for phase {}",
                ops.findings.len(),
                n_actions,
                state.current_phase.as_str()
            )
        },
    }
}

/// Apply repair plan (human `--yes`). Only phase-allowed commands.
pub fn repair_apply(state_path: &Path, yes: bool, runner: &dyn ArgvRunner) -> RepairReport {
    let mut plan = repair_plan(state_path, runner);
    if !yes {
        plan.status = "planned";
        plan.message = format!("{} — re-run with --yes to apply", plan.message);
        return plan;
    }

    let mut results = Vec::new();
    let mut any_fail = false;
    for a in &plan.actions {
        if a.command.contains('<') {
            results.push(RepairResult {
                id: a.id.clone(),
                attempted: false,
                ok: false,
                exit_code: None,
                detail: "template command requires operator alias/args".into(),
            });
            continue;
        }
        let argv = split_command(&a.command);
        let out = runner.run(&argv);
        if let Some(err) = &out.spawn_error {
            any_fail = true;
            results.push(RepairResult {
                id: a.id.clone(),
                attempted: true,
                ok: false,
                exit_code: None,
                detail: err.clone(),
            });
            continue;
        }
        let ok = out.success();
        if !ok {
            any_fail = true;
        }
        let detail = if out.stderr.trim().is_empty() {
            out.stdout.chars().take(300).collect()
        } else {
            out.stderr.chars().take(300).collect()
        };
        results.push(RepairResult {
            id: a.id.clone(),
            attempted: true,
            ok,
            exit_code: out.exit_code,
            detail,
        });
    }

    let mut state = load_state(state_path);
    state.last_apply_at_unix_ms = Some(now_ms());
    state.updated_at_unix_ms = now_ms();
    state.notes.push(format!(
        "repair_apply n={} fail={} at {}",
        results.len(),
        any_fail,
        state.updated_at_unix_ms
    ));
    if let Err(error) = save_state(state_path, &state) {
        any_fail = true;
        results.push(RepairResult {
            id: "lifecycle_state_persisted".into(),
            attempted: true,
            ok: false,
            exit_code: None,
            detail: error,
        });
    }

    plan.mutates = true;
    plan.results = results;
    plan.status = if any_fail { "degraded" } else { "ok" };
    plan.message = if any_fail {
        "repair apply completed with failures — see results".into()
    } else {
        "repair apply completed".into()
    };
    plan
}

fn classify_command(cmd: &str) -> MutationClass {
    if cmd.contains("deps upgrade") {
        MutationClass::DepsScripted
    } else if cmd.contains("inference") {
        MutationClass::InferenceRuntime
    } else if cmd.contains("bootstrap") {
        MutationClass::HostBootstrap
    } else if cmd.contains("network") || cmd.contains("fleet") {
        MutationClass::FleetNetwork
    } else {
        MutationClass::ObserveOnly
    }
}

fn command_mutates(cmd: &str) -> bool {
    cmd.contains("upgrade")
        || cmd.contains("inference-start")
        || cmd.contains("bootstrap")
        || cmd.contains(" apply")
        || cmd.contains("gate --yes")
}
