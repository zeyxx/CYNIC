//! Operator projection of stable CYNIC contract failures.

use std::process::ExitCode;

use cynic_protocol::{CYNIC_ERROR_CODES, CynicError};
use serde::Serialize;

use crate::cli_json::emit_pretty_json;

#[derive(Serialize)]
struct ErrorCatalogReport {
    schema: &'static str,
    errors: Vec<CynicError>,
}

pub(crate) fn execute_operator_errors(json: bool) -> ExitCode {
    let report = ErrorCatalogReport {
        schema: "cynic.operator_error_catalog.v1",
        errors: CYNIC_ERROR_CODES
            .iter()
            .copied()
            .map(CynicError::new)
            .collect(),
    };
    if json {
        return emit_pretty_json(&report);
    }
    println!("CYNIC error catalogue");
    for error in report.errors {
        println!("{}  {}", error.code, error.operator_action);
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use cynic_protocol::CYNIC_ERROR_CODES;

    #[test]
    fn catalogue_exposes_build_network_and_breakglass_contracts() {
        let codes: Vec<_> = CYNIC_ERROR_CODES.iter().map(|code| code.code()).collect();
        assert!(codes.contains(&"CYNIC-BLD-001"));
        assert!(codes.contains(&"CYNIC-NET-001"));
        assert!(codes.contains(&"CYNIC-BG-001"));
    }
}
