use std::process::ExitCode;

use crate::emit_json;

pub(crate) fn execute_resolver_plan(json: bool, hosts: Vec<String>) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }

    let report = cynic_host::runtime_resolver_plan::build_runtime_resolver_plan(&hosts);
    emit_json(&report, false)
}
