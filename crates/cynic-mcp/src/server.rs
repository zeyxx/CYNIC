//! MCP tool implementations. Each `#[tool]` below is a real executor with
//! live read-back: it observes the substrate and returns evidence, or a
//! typed error. No canned successes.
//!
//! Slice 1 is read-only and in-process. Every tool added later registers the
//! same way: the tool attribute macro on a method inside the tool_router
//! impl block.

use rmcp::handler::server::wrapper::Json;
use rmcp::{schemars, tool, tool_router};
use serde::Serialize;

/// Identity returned by `cynic_mcp_ping`. Crate facts only — contract
/// agreement stays at the session-handshake level.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct PingOutput {
    /// Server name.
    pub server: String,
    /// Crate version.
    pub version: String,
}

/// Slice-1 tools: read-only, in-process, no admission required.
#[derive(Clone, Debug, Default)]
pub struct CynicMcpServer;

#[tool_router(server_handler)]
impl CynicMcpServer {
    /// Liveness probe: return this server's identity.
    #[tool(
        name = "cynic_mcp_ping",
        description = "Liveness probe returning this MCP server identity"
    )]
    fn cynic_mcp_ping(&self) -> Json<PingOutput> {
        Json(PingOutput {
            server: "cynic-mcp".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    /// Report the MCP protocol versions this server speaks, newest first.
    #[tool(
        name = "cynic_protocol_versions",
        description = "Report the MCP protocol versions this server speaks, newest first"
    )]
    fn cynic_protocol_versions(&self) -> Json<Vec<String>> {
        Json(
            cynic_protocol::mcp_projection::SUPPORTED_MCP_PROTOCOL_VERSIONS
                .iter()
                .map(|version| (*version).to_string())
                .collect(),
        )
    }

    /// Inventory the observed workspace members (`crates/*/Cargo.toml`).
    /// No filter: membership is observed, experimental status belongs to
    /// `infra/runtime-authority.json`, not to this tool.
    #[tool(
        name = "cynic_workspace_inventory",
        description = "Inventory the observed workspace crate members"
    )]
    fn cynic_workspace_inventory(&self) -> Json<Vec<String>> {
        Json(crate::workspace_members())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_reports_crate_identity() {
        let server = CynicMcpServer;
        let output = server.cynic_mcp_ping().0;
        assert_eq!(output.server, "cynic-mcp");
        assert!(!output.version.is_empty());
    }

    #[test]
    fn protocol_versions_match_projection_law() {
        let server = CynicMcpServer;
        let versions = (server.cynic_protocol_versions()).0;
        assert_eq!(versions.first().map(String::as_str), Some("2026-07-28"));
        assert!(versions.contains(&"2025-11-25".to_string()));
    }

    #[test]
    fn inventory_starts_with_protocol() {
        let server = CynicMcpServer;
        let members = (server.cynic_workspace_inventory()).0;
        assert!(!members.is_empty());
        assert_eq!(members[0], "cynic-protocol");
    }
}
