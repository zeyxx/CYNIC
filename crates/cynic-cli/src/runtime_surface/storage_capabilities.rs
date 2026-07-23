use std::process::ExitCode;

use crate::emit_json;

pub(crate) fn execute_storage_capabilities(json: bool, url: Option<&str>) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }

    let endpoint = url
        .map(str::to_string)
        .or_else(|| std::env::var("SURREALDB_URL").ok())
        .unwrap_or_else(|| "http://localhost:8000".into());
    let report = cynic_host::storage_capabilities::observe_storage_capabilities(&endpoint);
    emit_json(&report, report.status == "blocked")
}
