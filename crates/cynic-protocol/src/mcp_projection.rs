//! Pure MCP projection types: the vocabulary a future stdio transport will
//! project, without owning tool logic, session state, or transport.
//!
//! Tool authority stays in `infra/agent-exposure.json`; contract
//! compatibility stays in `runtime_compatibility`. This module only names the
//! shapes both sides must agree on before any wire exists.

use serde::{Deserialize, Serialize};

pub const MCP_PROJECTION_SCHEMA: &str = "cynic.mcp_projection.v1";

/// Protocol versions the projection speaks, newest first. No new dependency
/// may extend this list without a conformance read-back.
pub const SUPPORTED_MCP_PROTOCOL_VERSIONS: [&str; 2] = ["2026-07-28", "2025-11-25"];

pub fn supports_mcp_protocol(version: &str) -> bool {
    SUPPORTED_MCP_PROTOCOL_VERSIONS.contains(&version)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolMutability {
    ReadOnly,
    Mutating,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub domain: String,
    pub mutability: McpToolMutability,
    /// Pointer to the input schema document, not the schema itself: the
    /// canonical tool table lives outside this crate.
    pub input_schema_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerDiscovery {
    pub schema: String,
    pub contract_id: String,
    pub contract_revision: u64,
    pub protocol_versions: Vec<String>,
    pub capabilities: Vec<String>,
    pub instructions: String,
}

/// Stamp a discovery document from the desired contract. Versions come from
/// the supported list only; callers supply contract identity and capabilities.
pub fn discover_mcp_server(
    contract_id: &str,
    contract_revision: u64,
    capabilities: Vec<String>,
    instructions: &str,
) -> McpServerDiscovery {
    McpServerDiscovery {
        schema: MCP_PROJECTION_SCHEMA.to_string(),
        contract_id: contract_id.to_string(),
        contract_revision,
        protocol_versions: SUPPORTED_MCP_PROTOCOL_VERSIONS
            .iter()
            .map(|version| (*version).to_string())
            .collect(),
        capabilities,
        instructions: instructions.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiates_only_supported_protocol_versions() {
        assert!(supports_mcp_protocol("2026-07-28"));
        assert!(supports_mcp_protocol("2025-11-25"));
        assert!(!supports_mcp_protocol("2024-11-05"));
        assert!(!supports_mcp_protocol(""));
    }

    #[test]
    fn discovery_stamps_supported_versions_newest_first() {
        let discovery = discover_mcp_server(
            "cynic.agent_mcp",
            8,
            vec!["cynic_runtime_surface".to_string()],
            "read-only bootstrap",
        );

        assert_eq!(discovery.schema, MCP_PROJECTION_SCHEMA);
        assert_eq!(discovery.contract_revision, 8);
        assert_eq!(
            discovery.protocol_versions,
            vec!["2026-07-28".to_string(), "2025-11-25".to_string()]
        );
    }

    #[test]
    fn tool_definition_round_trips_through_json() {
        let definition = McpToolDefinition {
            name: "cynic_runtime_surface".to_string(),
            description: "read-only runtime surface".to_string(),
            domain: "runtime".to_string(),
            mutability: McpToolMutability::ReadOnly,
            input_schema_ref: "infra/agent-exposure.json".to_string(),
        };

        let back: McpToolDefinition =
            serde_json::from_str(&serde_json::to_string(&definition).expect("serialize"))
                .expect("deserialize");
        assert_eq!(back, definition);
    }
}
