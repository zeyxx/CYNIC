use std::path::Path;
use std::process::ExitCode;

use crate::host_exec_cli::args::{declared_services, host_exec_policy_path};
use crate::host_exec_cli::io::{emit_json, read_stdin_json};
use crate::host_exec_policy::{HostExecPolicyDocument, HostExecRequest};

pub(crate) fn execute_host_exec_validate_request(args: &[String]) -> Result<ExitCode, String> {
    let policy_path = host_exec_policy_path(args);
    let declared_services = declared_services(args);
    let payload = read_stdin_json()?;
    let request: HostExecRequest = serde_json::from_value(payload)
        .map_err(|error| format!("invalid host exec request: {error}"))?;
    let policy = HostExecPolicyDocument::load(Path::new(&policy_path))?;
    let validation = policy.validate_request(&request, &declared_services);
    emit_json(&validation, !validation.ok)
}
