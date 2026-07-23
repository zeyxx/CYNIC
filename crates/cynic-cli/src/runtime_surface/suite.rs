use std::process::ExitCode;

use crate::{emit_json, fail, registry_path, runtime_suite};

pub(crate) fn execute_suite(json: bool, registry: Option<&str>, profile: &str) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let registry = registry_path(registry.map(|value| value.to_string()));
    match runtime_suite::build_runtime_execution_suite(&registry, profile) {
        Ok(report) => emit_json(&report, false),
        Err(error) => fail(error),
    }
}

pub(crate) fn execute_plan(
    json: bool,
    registry: Option<&str>,
    profile: &str,
    proof: Option<&str>,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let registry = registry_path(registry.map(|value| value.to_string()));
    match runtime_suite::build_runtime_execution_plan(&registry, profile, proof) {
        Ok(report) => emit_json(&report, false),
        Err(error) => fail(error),
    }
}
