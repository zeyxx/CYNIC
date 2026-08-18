use std::process::ExitCode;

use crate::{emit_json, fail};

pub(crate) fn execute_harness_matrix(json: bool, registry_path: Option<&str>) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let registry = registry_path
        .filter(|path| !path.trim().is_empty())
        .unwrap_or("infra/registry.json");
    match cynic_host::build_runner_harness::build_runner_harness_plan(registry, None) {
        Ok(report) => emit_json(&report, false),
        Err(error) => fail(error),
    }
}
