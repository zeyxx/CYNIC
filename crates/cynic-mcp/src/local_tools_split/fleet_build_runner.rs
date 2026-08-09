//! Local MCP adapter: native build_runner selection + status projection for
//! citizen agents of the CYNIC republic.
//!
//! Thin projection over `cynic fleet build-runner --json` (the canonical CLI
//! surface). The reconcile logic lives in `cynic_node::domain::fleet_truth`;
//! this adapter only forwards the request and normalizes the JSON read-back.

use cynic_host::ports::argv_execution::{ArgvExecutionOutput, ArgvExecutionPort, ArgvExecutionRequest};
use serde_json::Value;
use std::time::Duration;

/// Select the build_runner node for the next admitted build, with explicit
/// graceful-degradation reporting. Thin wrapper over the canonical CLI surface.
///
/// `preferred` overrides the announced build_runner node. `registry` optionally
/// points at a non-default registry path. `allow_local_fallback` controls
/// whether a node-less fleet degrades to `degraded_local` instead of `blocked`.
pub async fn run_build_runner_select(
    runner: &dyn ArgvExecutionPort,
    preferred: Option<&str>,
    allow_local_fallback: bool,
    registry: Option<&str>,
) -> Value {
    let mut argv = vec![
        "fleet".to_string(),
        "build-runner".to_string(),
        "--json".to_string(),
    ];
    if let Some(preferred) = preferred {
        argv.push("--preferred".to_string());
        argv.push(preferred.to_string());
    }
    if !allow_local_fallback {
        argv.push("--no-local-fallback".to_string());
    }
    if let Some(registry) = registry {
        argv.push("--registry".to_string());
        argv.push(registry.to_string());
    }

    let raw = run_cynic_cmd(runner, &argv).await;
    if !raw.exit_code.map(|c| c == 0).unwrap_or(false) {
        return serde_json::json!({
            "schema": "cynic.build_runner_selection.v1",
            "status": "blocked",
            "error": raw.stderr.trim(),
            "command": format!("cynic {}", argv.join(" ")),
        });
    }

    // The CLI already emits a machine-readable SelectedBuildRunner document.
    match serde_json::from_str::<Value>(&raw.stdout) {
        Ok(value) => value,
        Err(_) => serde_json::json!({
            "schema": "cynic.build_runner_selection.v1",
            "status": "blocked",
            "error": "invalid_json_readback",
            "raw": raw.stdout,
        }),
    }
}

/// Read-only status of the announced build_runner node (no reconcile).
/// Reuses `run_build_runner_select` with the preferred node pinned.
pub async fn run_build_runner_status(
    runner: &dyn ArgvExecutionPort,
    preferred: Option<&str>,
    registry: Option<&str>,
) -> Value {
    run_build_runner_select(runner, preferred, true, registry).await
}

async fn run_cynic_cmd(runner: &dyn ArgvExecutionPort, argv: &[String]) -> ArgvExecutionOutput {
    let request = ArgvExecutionRequest {
        argv: argv.to_vec(),
        current_dir: None,
        stdin: None,
        timeout: Duration::from_secs(120),
    };
    runner.execute(request).await
}
