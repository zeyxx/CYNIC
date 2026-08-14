use std::process::ExitCode;

use cynic_host::registry_observation::project_registry_observation;

use crate::{emit_json, fail, registry_path};

pub(crate) fn execute_registry_observation(
    json: bool,
    registry: Option<String>,
    observation: &str,
    apply: bool,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let registry = registry_path(registry);
    match project_registry_observation(&registry, observation, apply) {
        Ok(report) => emit_json(&report, report.status != "applied"),
        Err(error) => fail(error),
    }
}
