//! In-tree Model Context Protocol (MCP) server surface.

use clap::Subcommand;
use rmcp::ServiceExt;
use std::process::ExitCode;

#[derive(Subcommand, Debug, Clone)]
pub enum McpCommand {
    /// Run the sovereign CYNIC MCP server over standard I/O (stdio).
    Serve,
}

pub async fn dispatch(cmd: McpCommand) -> ExitCode {
    match cmd {
        McpCommand::Serve => {
            let server = cynic_mcp::server::CynicMcpServer::default();
            match server.serve(rmcp::transport::io::stdio()).await {
                Ok(running) => {
                    if let Err(e) = running.waiting().await {
                        eprintln!("cynic mcp error: {e}");
                        ExitCode::FAILURE
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("cynic mcp startup error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
