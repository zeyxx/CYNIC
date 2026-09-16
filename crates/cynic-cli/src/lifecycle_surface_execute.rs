use std::process::ExitCode;

use crate::cli_exit::{success_if_status, success_if_status_in};
use crate::cli_json::emit_pretty_json;
use crate::lifecycle_surface_context::{runner, state_path};
use crate::lifecycle_surface_render::{
    print_apply_human, print_gate_human, print_kernel_release_human, print_ops_human,
    print_plan_human, print_promote_human, print_repair_human, print_status_human,
};
use cynic_lifecycle::{
    ApplyReport, KernelReleaseReport, RepairReport, apply_phase, ops_report, plan_report, promote,
    reconcile_kernel_release, repair_apply, repair_plan, run_gates, status_report,
};

pub(crate) fn execute_evidence(args: Vec<String>) -> ExitCode {
    match cynic_host::lifecycle::evidence_cli::run(args) {
        Ok(payload) => emit_pretty_json(&payload),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_status(json: bool, state: Option<String>) -> ExitCode {
    let path = state_path(state);
    let report = status_report(&path);
    if json {
        return emit_pretty_json(&report);
    }
    print_status_human(&report);
    ExitCode::SUCCESS
}

pub(crate) fn execute_plan(json: bool, state: Option<String>) -> ExitCode {
    let path = state_path(state);
    let report = plan_report(&path);
    if json {
        return emit_pretty_json(&report);
    }
    print_plan_human(&report);
    ExitCode::SUCCESS
}

pub(crate) fn execute_gate(json: bool, yes: bool, state: Option<String>) -> ExitCode {
    let path = state_path(state);
    let report = run_gates(&path, yes, &runner());
    if json {
        let code = emit_pretty_json(&report);
        return success_if_status_in(&report.status, &["ok", "planned"], code);
    }
    print_gate_human(&report, yes);
    success_if_status_in(&report.status, &["ok", "planned"], ExitCode::SUCCESS)
}

pub(crate) fn execute_promote(
    json: bool,
    to: Option<String>,
    yes: bool,
    state: Option<String>,
) -> ExitCode {
    let path = state_path(state);
    match promote(&path, to.as_deref(), yes) {
        Ok(report) => {
            if json {
                let code = emit_pretty_json(&report);
                return success_if_status_in(&report.status, &["ok", "planned"], code);
            }
            print_promote_human(&report);
            success_if_status_in(&report.status, &["ok", "planned"], ExitCode::SUCCESS)
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_report(json: bool, state: Option<String>) -> ExitCode {
    let path = state_path(state);
    let report = ops_report(&path, &runner());
    if json {
        let code = emit_pretty_json(&report);
        return success_if_status(&report.status, "ok", code);
    }
    print_ops_human(&report);
    success_if_status(&report.status, "ok", ExitCode::SUCCESS)
}

pub(crate) fn execute_repair(json: bool, yes: bool, state: Option<String>) -> ExitCode {
    let path = state_path(state);
    let runner = runner();
    let report = if yes {
        repair_apply(&path, true, &runner)
    } else {
        repair_plan(&path, &runner)
    };
    if json {
        let code = emit_pretty_json(&report);
        return exit_for_repair(&report, code);
    }
    print_repair_human(&report, yes);
    exit_for_repair(&report, ExitCode::SUCCESS)
}

pub(crate) fn execute_apply(json: bool, yes: bool, state: Option<String>) -> ExitCode {
    let path = state_path(state);
    let report = apply_phase(&path, yes, &runner());
    if json {
        let code = emit_pretty_json(&report);
        return exit_for_apply(&report, code);
    }
    print_apply_human(&report, yes);
    exit_for_apply(&report, ExitCode::SUCCESS)
}

pub(crate) fn execute_kernel_release(
    spec: String,
    json: bool,
    yes: bool,
    state: Option<String>,
) -> ExitCode {
    let path = state_path(state);
    let report = reconcile_kernel_release(&path, std::path::Path::new(&spec), yes, &runner());
    if json {
        let code = emit_pretty_json(&report);
        return exit_for_kernel_release(&report, code);
    }
    print_kernel_release_human(&report, yes);
    exit_for_kernel_release(&report, ExitCode::SUCCESS)
}

fn exit_for_repair(report: &RepairReport, ok_code: ExitCode) -> ExitCode {
    success_if_status_in(&report.status, &["ok", "planned"], ok_code)
}

fn exit_for_apply(report: &ApplyReport, ok_code: ExitCode) -> ExitCode {
    success_if_status_in(&report.status, &["ok", "planned"], ok_code)
}

fn exit_for_kernel_release(report: &KernelReleaseReport, ok_code: ExitCode) -> ExitCode {
    success_if_status_in(report.status.as_str(), &["ok", "planned"], ok_code)
}
