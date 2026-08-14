use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cynic_host::update_lifecycle::{
    LocalArgvRunner, default_state_path, reconcile_runtime_artifact_release,
};

pub fn execute_reconcile(spec_path: &str, lifecycle_state: Option<&str>, yes: bool) -> ExitCode {
    let state_path = lifecycle_state
        .map(PathBuf::from)
        .unwrap_or_else(default_state_path);
    let report = reconcile_runtime_artifact_release(
        &state_path,
        Path::new(spec_path),
        yes,
        &LocalArgvRunner,
    );
    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            let _ = io::stderr().write_all(
                format!("runtime artifact release serialization failed: {error}\n").as_bytes(),
            );
            return ExitCode::FAILURE;
        }
    }
    match report.status.as_str() {
        "ok" | "planned" => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}
