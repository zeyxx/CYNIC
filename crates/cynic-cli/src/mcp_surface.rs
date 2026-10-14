//! In-tree Model Context Protocol (MCP) server surface.

use clap::Subcommand;
use std::process::{Command, ExitCode};

#[derive(Subcommand, Debug, Clone)]
pub enum McpCommand {
    /// Run the sovereign CYNIC MCP server over standard I/O (stdio).
    Serve,
}

pub async fn dispatch(cmd: McpCommand) -> ExitCode {
    match cmd {
        McpCommand::Serve => {
            let mcp_bin =
                std::env::var("CYNIC_MCP_BIN").unwrap_or_else(|_| "cynic-mcp".to_string());
            match Command::new(&mcp_bin).status() {
                Ok(status) if status.success() => ExitCode::SUCCESS,
                Ok(status) => {
                    let code = status.code().unwrap_or(1);
                    ExitCode::from(code as u8)
                }
                Err(e) => {
                    eprintln!("cynic mcp execution error ({mcp_bin}): {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
