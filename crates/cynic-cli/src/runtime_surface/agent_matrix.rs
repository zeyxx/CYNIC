use std::process::ExitCode;

use crate::emit_json;

pub(crate) fn execute_agent_matrix(json: bool) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }

    let report = cynic_host::agent_matrix::build_agent_matrix_report();
    emit_json(&report, false)
}
