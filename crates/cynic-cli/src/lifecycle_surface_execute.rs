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
    registry: String,
) -> ExitCode {
    let path = state_path(state);
    let report = reconcile_kernel_release(&path, std::path::Path::new(&spec), yes, &runner());
    if json {
        let code = emit_pretty_json(&report);
        let exit = exit_for_kernel_release(&report, code);
        if exit == ExitCode::SUCCESS {
            return publish_after_release(&spec, &registry);
        }
        return exit;
    }
    print_kernel_release_human(&report, yes);
    let exit = exit_for_kernel_release(&report, ExitCode::SUCCESS);
    if exit == ExitCode::SUCCESS {
        return publish_after_release(&spec, &registry);
    }
    exit
}

/// Publish the staged artifact when the spec declares a publication target.
///
/// No-op (SUCCESS) for local-only specs. Reads the spec back from disk so
/// the decision rests on the same desired state that was reconciled.
/// Fail-closed: publication errors surface as FAILURE, never masked.
fn publish_after_release(spec_path: &str, registry: &str) -> ExitCode {
    let raw = match std::fs::read_to_string(spec_path) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("release publication failed: read {spec_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let spec: cynic_lifecycle::kernel_release::types::KernelReleaseSpec =
        match serde_json::from_str(&raw) {
            Ok(spec) => spec,
            Err(error) => {
                eprintln!("release publication failed: parse {spec_path}: {error}");
                return ExitCode::FAILURE;
            }
        };
    if spec.publication.is_none() {
        return ExitCode::SUCCESS;
    }
    let proof_sha = env!("CYNIC_BUILD_PROOF_SHA");
    if proof_sha.is_empty() {
        eprintln!("release publication failed: no build proof baked into this binary");
        return ExitCode::FAILURE;
    }
    let http_target = match provider_control_endpoint(registry) {
        Some(endpoint) => endpoint,
        None => {
            eprintln!("release publication failed: no provider_control endpoint in {registry}");
            return ExitCode::FAILURE;
        }
    };
    let ssh_target = spec
        .publication
        .as_ref()
        .map(|t| t.endpoint.clone())
        .unwrap_or_default();
    let publisher =
        cynic_host::adapters::artifact_publisher_http::HttpArtifactPublisher::new(&http_target);
    match cynic_lifecycle::kernel_release::publish_released_artifact(
        &spec,
        proof_sha,
        &http_target,
        &ssh_target,
        &publisher,
    ) {
        Ok(Some(published)) => {
            println!(
                "published {} -> {}",
                published.proof_sha, published.published_uri
            );
            ExitCode::SUCCESS
        }
        Ok(None) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("release publication failed: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Resolve the provider_control HTTP endpoint from the registry.
fn provider_control_endpoint(registry_path: &str) -> Option<String> {
    let content = std::fs::read_to_string(registry_path).ok()?;
    let doc: serde_json::Value = serde_json::from_str(&content).ok()?;
    doc.get("provider_control")?
        .get("endpoint")?
        .as_str()
        .map(|s| s.trim_end_matches('/').to_string())
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
