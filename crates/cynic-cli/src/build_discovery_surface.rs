//! Build discovery thin CLI surface — clap parse -> adapter observe -> JSON print.
//! No policy; delegates to Local/RemoteBuildDiscoveryAdapter.

use std::process::ExitCode;
use std::sync::Arc;

use cynic_host::adapters::local_build_discovery::LocalBuildDiscoveryAdapter;
use cynic_host::adapters::remote_build_discovery::RemoteBuildDiscoveryAdapter;
use cynic_host::domain::build_discovery::port::BuildDiscoveryObservationPort;
use cynic_lifecycle::LocalArgvRunner;

use crate::cli_json::emit_pretty_json;
use crate::emit_json;

pub(crate) fn execute_build_discover(json: bool, kind: &str, target: Option<&str>) -> ExitCode {
    let report = match kind {
        "local" => {
            let adapter = LocalBuildDiscoveryAdapter;
            match adapter.observe() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("build discovery failed: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
        "remote" => {
            let ssh_target = match target {
                Some(t) if !t.is_empty() => t.to_string(),
                _ => {
                    eprintln!("--kind remote requires --target <ssh-target>");
                    return ExitCode::FAILURE;
                }
            };
            let runner = Arc::new(LocalArgvRunner);
            let adapter = match RemoteBuildDiscoveryAdapter::new(runner, ssh_target) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("invalid ssh target: {e}");
                    return ExitCode::FAILURE;
                }
            };
            match adapter.observe() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("remote build discovery failed: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
        other => {
            eprintln!("unknown --kind '{other}': expected local|remote");
            return ExitCode::FAILURE;
        }
    };

    if json {
        emit_json(&report, false)
    } else {
        // human fallback, but JSON is canonical
        emit_pretty_json(&report)
    }
}
