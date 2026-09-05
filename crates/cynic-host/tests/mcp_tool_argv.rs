//! M2 argv contract: the tool adapter appends `--json` to the tool argv,
//! passes results through unchanged, and never masks a spawn failure.

use cynic_host::mcp_tool::{CliArgvToolAdapter, McpToolPort};
use cynic_host::update_lifecycle::{ArgvOutput, MapArgvRunner};

fn stubbed_runner() -> MapArgvRunner {
    MapArgvRunner {
        responses: vec![(
            vec![
                "cynic".to_string(),
                "runtime".to_string(),
                "capacity".to_string(),
                "--json".to_string(),
            ],
            ArgvOutput {
                exit_code: Some(0),
                stdout: "{\"status\":\"ok\"}".to_string(),
                stderr: String::new(),
                spawn_error: None,
            },
        )],
    }
}

#[test]
fn tool_call_runs_cli_argv_with_json_flag() {
    let adapter = CliArgvToolAdapter::new("cynic");
    let output = adapter.execute(&stubbed_runner(), &["runtime", "capacity"]);

    assert!(output.spawn_error.is_none());
    assert_eq!(output.exit_code, Some(0));
    assert_eq!(output.stdout, "{\"status\":\"ok\"}");
}

#[test]
fn unknown_tool_surfaces_spawn_error_without_success() {
    let adapter = CliArgvToolAdapter::new("cynic");
    let output = adapter.execute(&MapArgvRunner::default(), &["nope"]);

    assert!(output.spawn_error.is_some());
    assert!(!output.success());
}
