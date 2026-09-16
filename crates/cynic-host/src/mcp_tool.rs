//! Tool execution behind an injected argv runner.
//!
//! This module is transport only: it runs `cynic <tool...> --json` through
//! the provided runner and passes the result through unchanged. Admission,
//! capability checks, and the canonical tool table live outside this module
//! (`infra/agent-exposure.json` plus the protocol projection types). A spawn
//! failure is returned as-is, never upgraded into a success.

use cynic_lifecycle::{ArgvOutput, ArgvRunner};

/// Execution port for one MCP tool call over the local CLI surface.
pub trait McpToolPort {
    fn execute(&self, runner: &dyn ArgvRunner, tool_argv: &[&str]) -> ArgvOutput;
}

/// CLI-backed tool adapter. The `--json` flag is appended so every call
/// yields machine-readable output for the MCP projection.
#[derive(Debug, Clone)]
pub struct CliArgvToolAdapter {
    cli_binary: String,
}

impl CliArgvToolAdapter {
    pub fn new(cli_binary: &str) -> Self {
        Self {
            cli_binary: cli_binary.to_string(),
        }
    }
}

impl McpToolPort for CliArgvToolAdapter {
    fn execute(&self, runner: &dyn ArgvRunner, tool_argv: &[&str]) -> ArgvOutput {
        let owned: Vec<String> = std::iter::once(self.cli_binary.clone())
            .chain(tool_argv.iter().map(|arg| (*arg).to_string()))
            .chain(std::iter::once("--json".to_string()))
            .collect();
        let args: Vec<&str> = owned.iter().map(String::as_str).collect();
        runner.run(&args)
    }
}
