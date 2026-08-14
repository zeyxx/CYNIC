use std::process::ExitCode;

use crate::emit_json;

pub(crate) fn execute_execution_context(json: bool, hosts: &[String]) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }

    let report = cynic_host::runtime_execution_context::observe_runtime_execution_context(hosts);
    emit_json(&report, report.status != "ready")
}
