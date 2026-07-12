use std::process::ExitCode;

use cynic_protocol::RuntimeProofBundle;

use crate::bootstrap_surface::{ProfileArgs, selected_host_context};
use crate::{emit_json, fail};

pub(crate) fn execute_harness(json: bool, kind: Option<String>) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    match cynic_host::runtime_harness::runtime_harness_catalog(kind.as_deref()) {
        Ok(report) => emit_json(&report, false),
        Err(error) => fail(error),
    }
}

pub(crate) fn execute_conformance(json: bool, profile: ProfileArgs) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let (selected, resolution) = match selected_host_context(&profile) {
        Ok(value) => value,
        Err(error) => return fail(error),
    };
    match cynic_host::conformance::observe_host_conformance(selected, resolution) {
        Ok(report) => emit_json(&report, report.status != "qualified"),
        Err(error) => fail(error.to_string()),
    }
}

pub(crate) fn execute_metrics(json: bool, profile: ProfileArgs, proof: Option<String>) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let (selected, resolution) = match selected_host_context(&profile) {
        Ok(value) => value,
        Err(error) => return fail(error),
    };
    let external_proof = match proof {
        Some(path) => match load_runtime_proof_bundle(&path) {
            Ok(bundle) => Some(bundle),
            Err(error) => return fail(error),
        },
        None => None,
    };
    match cynic_host::conformance::observe_host_conformance(selected, resolution) {
        Ok(conformance) => {
            let metrics = cynic_host::runtime_metrics::host_runtime_metrics_with_external_proof(
                &conformance,
                external_proof.as_ref(),
            );
            emit_json(&metrics, false)
        }
        Err(error) => fail(error.to_string()),
    }
}

pub(crate) fn execute_repo_access(json: bool, profile: Option<String>) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let report = match profile {
        Some(path) => cynic_host::repo_access::repo_access_status(
            std::path::Path::new(&path),
            cynic_host::repo_access::current_project_root().as_deref(),
        ),
        None => cynic_host::repo_access::current_repo_access_status(),
    };
    emit_json(&report, false)
}

fn load_runtime_proof_bundle(path: &str) -> Result<RuntimeProofBundle, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read runtime proof bundle {path}: {error}"))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse runtime proof bundle {path}: {error}"))
}
