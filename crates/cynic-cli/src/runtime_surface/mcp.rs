use std::process::ExitCode;

use cynic_host::{mcp_runtime::observe_mcp_runtime, update_lifecycle::LocalArgvRunner};

use crate::emit_json;

pub(crate) fn execute_mcp_observation(json: bool, binary: &str) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let observation = observe_mcp_runtime(binary, &LocalArgvRunner);
    emit_json(&observation, observation.status != "observed")
}
