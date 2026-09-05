//! M1 compatibility matrix for the installed MCP artifact.
//!
//! Four artifact states plus the exit-status contract, proven through the
//! existing owners only (`cynic_host::mcp_runtime::observe_mcp_runtime` over
//! an injected `MapArgvRunner`). No binary is built, no server is started,
//! nothing mutates: every observation must report `mutates == false`.
//!
//! Fixtures live in `fixtures/mcp_compat/` next to this target so the matrix
//! stays data, not code.

use std::fs;
use std::path::PathBuf;

use cynic_host::mcp_runtime::observe_mcp_runtime;
use cynic_host::update_lifecycle::{ArgvOutput, MapArgvRunner};

fn fixture(name: &str) -> String {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "mcp_compat",
        name,
    ]
    .iter()
    .collect();
    fs::read_to_string(&path).expect("read mcp compat fixture")
}

fn runner_for(
    stdout: String,
    exit_code: Option<i32>,
    spawn_error: Option<String>,
) -> MapArgvRunner {
    MapArgvRunner {
        responses: vec![(
            vec!["cynic-mcp".to_string(), "--contract-json".to_string()],
            ArgvOutput {
                exit_code,
                stdout,
                stderr: String::new(),
                spawn_error,
            },
        )],
    }
}

#[test]
fn compatible_artifact_is_observed_read_only() {
    let observed = observe_mcp_runtime(
        "cynic-mcp",
        &runner_for(fixture("compatible.json"), Some(0), None),
    );

    assert_eq!(observed.status, "observed");
    assert!(!observed.mutates);
    assert!(observed.blockers.is_empty());
    assert!(observed.required_actions.is_empty());
    assert!(observed.compatibility.is_some());
}

#[test]
fn revision6_artifact_is_blocked_incompatible() {
    let observed = observe_mcp_runtime(
        "cynic-mcp",
        &runner_for(fixture("incompatible_rev6.json"), Some(3), None),
    );

    assert_eq!(observed.status, "blocked");
    assert!(!observed.mutates);
    assert!(observed.blockers.is_empty());
    assert_eq!(observed.required_actions.len(), 3);
    let plan = observed.compatibility.expect("plan retained on block");
    assert_eq!(plan.observed_revision, Some(6));
    assert_eq!(plan.required_revision, 8);
}

#[test]
fn missing_binary_is_blocked_unobservable() {
    let observed = observe_mcp_runtime("missing-mcp", &MapArgvRunner::default());

    assert_eq!(observed.status, "blocked");
    assert!(!observed.mutates);
    assert_eq!(observed.blockers, vec!["mcp_runtime_artifact_unobservable"]);
    assert!(observed.compatibility.is_none());
}

#[test]
fn garbage_stdout_is_blocked_invalid() {
    let observed = observe_mcp_runtime(
        "cynic-mcp",
        &runner_for(fixture("invalid_stdout.txt"), Some(0), None),
    );

    assert_eq!(observed.status, "blocked");
    assert!(!observed.mutates);
    assert_eq!(
        observed.blockers,
        vec!["mcp_runtime_contract_report_invalid"]
    );
    assert!(observed.compatibility.is_none());
}

#[test]
fn compatible_plan_with_wrong_exit_is_blocked() {
    let observed = observe_mcp_runtime(
        "cynic-mcp",
        &runner_for(fixture("compatible.json"), Some(1), None),
    );

    assert_eq!(observed.status, "blocked");
    assert!(!observed.mutates);
    assert_eq!(
        observed.blockers,
        vec!["mcp_runtime_contract_exit_status_unexpected"]
    );
}
