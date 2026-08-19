use std::process::ExitCode;

use cynic_host::build::{BuildRequest, run_build};
use cynic_host::update_lifecycle::LocalArgvRunner;
use cynic_protocol::{SourceSnapshot, admit_build_input};

use crate::cli_json::emit_pretty_json;
use crate::fail;

pub(crate) fn execute_build(
    json: bool,
    source_snapshot_path: &str,
    artifact_path: &str,
    apply: bool,
    command: &[String],
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let raw = match std::fs::read_to_string(source_snapshot_path) {
        Ok(raw) => raw,
        Err(error) => return fail(format!("failed to read source snapshot: {error}")),
    };
    let snapshot = match serde_json::from_str::<SourceSnapshot>(&raw) {
        Ok(snapshot) => snapshot,
        Err(error) => return fail(format!("failed to parse source snapshot: {error}")),
    };
    let Some(source_commit) = snapshot.head_commit.clone() else {
        return fail("source snapshot has no HEAD commit".to_string());
    };
    let input = admit_build_input(&snapshot);
    let request = BuildRequest {
        input,
        source_commit,
        argv: command.to_vec(),
        artifact_path: artifact_path.to_string(),
        apply,
    };
    let runner = LocalArgvRunner;
    let report = run_build(&request, &runner);
    emit_pretty_json(&report)
}
