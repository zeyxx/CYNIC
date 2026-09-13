//! Phase-allowed mutations (deps upgrade, inference discover, …).
//!
//! Host bootstrap / fleet network are listed in plan/policy but never
//! auto-applied here — too wide a blast radius.

use std::path::Path;

use super::policy::phase_policy;
use super::port::{ArgvRunner, split_command};
use super::state_store::{load_state, save_state};
use super::time::now_ms;
use super::types::{
    ApplyReport, MutationClass, RepairAction, RepairResult, UPDATE_LIFECYCLE_APPLY_SCHEMA,
};

/// Plan or apply scripted mutations allowed by the current phase.
pub fn apply_phase(state_path: &Path, yes: bool, runner: &dyn ArgvRunner) -> ApplyReport {
    let state = load_state(state_path);
    let policy = phase_policy(&state.current_phase);
    let mut actions = Vec::new();

    for m in &policy.allowed_mutations {
        match m {
            MutationClass::DepsScripted => actions.push(RepairAction {
                id: "deps_upgrade".into(),
                command: "cynic deps upgrade --yes".into(),
                mutates: true,
                mutation_class: m.clone(),
                finding_ids: vec![],
                purpose: "apply scripted ecosystem upgrades under principal".into(),
            }),
            MutationClass::ObserveOnly => actions.push(RepairAction {
                id: "ops_report".into(),
                command: "cynic lifecycle report --json".into(),
                mutates: false,
                mutation_class: m.clone(),
                finding_ids: vec![],
                purpose: "refresh operational observation".into(),
            }),
            MutationClass::InferenceRuntime => actions.push(RepairAction {
                id: "inference_discover".into(),
                command: "cynic runtime inference-discover --json --apply".into(),
                mutates: true,
                mutation_class: m.clone(),
                finding_ids: vec![],
                purpose: "attach OpenAI-compat peers (optional ODS) into inference SSOT".into(),
            }),
            MutationClass::KernelRuntime => {
                // Applied only through `lifecycle kernel-release` with an explicit versioned spec.
            }
            MutationClass::HostBootstrap | MutationClass::FleetNetwork => {
                // Not auto-applied — too wide; visible via lifecycle plan only.
            }
        }
    }

    if !yes {
        return ApplyReport {
            schema: UPDATE_LIFECYCLE_APPLY_SCHEMA,
            status: "planned",
            phase: state.current_phase.as_str(),
            mutates: false,
            actions,
            results: vec![],
            message: "phase apply planned — re-run with --yes".into(),
        };
    }

    let mut results = Vec::new();
    let mut any_fail = false;
    for a in &actions {
        if a.command.contains('<') {
            results.push(RepairResult {
                id: a.id.clone(),
                attempted: false,
                ok: false,
                exit_code: None,
                detail: "skipped template".into(),
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
        if !ok && a.mutates {
            any_fail = true;
        }
        results.push(RepairResult {
            id: a.id.clone(),
            attempted: true,
            ok,
            exit_code: out.exit_code,
            detail: out.stdout.chars().take(200).collect(),
        });
    }

    let mut state = load_state(state_path);
    state.last_apply_at_unix_ms = Some(now_ms());
    state.updated_at_unix_ms = now_ms();
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

    ApplyReport {
        schema: UPDATE_LIFECYCLE_APPLY_SCHEMA,
        status: if any_fail { "degraded" } else { "ok" },
        phase: state.current_phase.as_str(),
        mutates: true,
        actions,
        results,
        message: if any_fail {
            "phase apply finished with failures".into()
        } else {
            "phase apply finished".into()
        },
    }
}
