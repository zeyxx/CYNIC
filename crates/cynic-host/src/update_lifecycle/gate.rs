//! Gate execution via ArgvRunner port.

use std::path::Path;

use super::policy::phase_policy;
use super::port::{ArgvRunner, json_status, split_command};
use super::state_store::{load_state, save_state};
use super::time::now_ms;
use super::types::{
    GateResult, LifecycleGateReport, UPDATE_LIFECYCLE_GATE_SCHEMA,
};

pub fn run_gates(
    state_path: &Path,
    execute: bool,
    runner: &dyn ArgvRunner,
) -> LifecycleGateReport {
    let mut state = load_state(state_path);
    let policy = phase_policy(&state.current_phase);
    let mut results = Vec::new();
    let mut all_required_ok = true;

    for g in &policy.gates {
        if !execute {
            results.push(GateResult {
                id: g.id,
                command: g.command,
                required: g.required,
                ran: false,
                ok: false,
                exit_code: None,
                detail: "planned only — pass --yes to execute gates".into(),
            });
            if g.required {
                all_required_ok = false;
            }
            continue;
        }

        let argv = split_command(g.command);
        let out = runner.run(&argv);
        if let Some(err) = &out.spawn_error {
            if g.required {
                all_required_ok = false;
            }
            results.push(GateResult {
                id: g.id,
                command: g.command,
                required: g.required,
                ran: true,
                ok: false,
                exit_code: None,
                detail: err.clone(),
            });
            continue;
        }

        let mut ok = out.success();
        if let Some(st) = json_status(&out.stdout) {
            ok = ok && st != "blocked" && st != "error";
        }
        if g.required && !ok {
            all_required_ok = false;
        }
        let detail = json_status(&out.stdout)
            .map(|st| format!("json status={st}"))
            .unwrap_or_else(|| {
                if ok {
                    "exit 0".into()
                } else {
                    format!(
                        "exit {:?}: {}",
                        out.exit_code,
                        out.stderr.chars().take(200).collect::<String>()
                    )
                }
            });
        results.push(GateResult {
            id: g.id,
            command: g.command,
            required: g.required,
            ran: true,
            ok,
            exit_code: out.exit_code,
            detail,
        });
    }

    state.last_gate_ok = Some(all_required_ok);
    state.last_gate_at_unix_ms = Some(now_ms());
    state.updated_at_unix_ms = now_ms();
    if execute {
        let _ = save_state(state_path, &state);
    }

    let mut next = vec![
        "cynic lifecycle status".into(),
        "cynic lifecycle plan".into(),
    ];
    if all_required_ok {
        if let Some(n) = state.current_phase.next() {
            next.push(format!("cynic lifecycle promote --to {} --yes", n.as_str()));
        }
    } else {
        next.push("fix failing gates then: cynic lifecycle gate --yes".into());
    }

    LifecycleGateReport {
        schema: UPDATE_LIFECYCLE_GATE_SCHEMA,
        status: if !execute {
            "planned"
        } else if all_required_ok {
            "ok"
        } else {
            "blocked"
        },
        phase: state.current_phase.as_str(),
        all_required_ok,
        results,
        next,
    }
}
