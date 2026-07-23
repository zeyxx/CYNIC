use std::process::ExitCode;

use crate::emit_json;

pub(crate) fn execute_network_preflight(json: bool, hosts: &[String]) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }

    let report = cynic_host::runtime_network_preflight::observe_runtime_network_preflight(hosts);
    emit_json(&report, report.status != "ready")
}
