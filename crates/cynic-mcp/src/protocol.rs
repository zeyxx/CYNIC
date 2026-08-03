//! MCP protocol-profile projections that are independent of transport adapters.
//!
//! The runtime currently serves the legacy stdio compatibility path.  This
//! module exposes the 2026-07-28 discovery contract without claiming that the
//! stateless Streamable HTTP implementation is complete.

use serde_json::{Value, json};

pub const TARGET_REVISION: &str = "2026-07-28";
pub const DISCOVERY_METHOD: &str = "server/discover";

/// Return the active CYNIC tool definitions from the exposure SSOT.
///
/// Compatibility checks, discovery, and surface projections must all consume
/// this one decision primitive instead of rebuilding the registry traversal.
pub fn exposed_tool_definitions(exposure: &Value) -> Vec<Value> {
    exposure
        .get("mcp_servers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|server| server.get("name").and_then(Value::as_str) == Some("cynic"))
        .and_then(|server| server.get("tools"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .cloned()
        .collect()
}

pub fn exposed_tool_names(exposure: &Value) -> Vec<String> {
    exposed_tool_definitions(exposure)
        .into_iter()
        .filter_map(|tool| {
            tool.get("name")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .collect()
}

/// Build the machine-readable discovery result from the same exposure SSOT
/// used by the runtime contract and the client projections.
pub fn discovery_result(exposure: &Value, request_meta_observed: bool, transport: &str) -> Value {
    let profile = exposure
        .get("mcp_protocol_profile")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let tools = Value::Array(exposed_tool_definitions(exposure));
    let implementation_status = profile
        .get("implementation_status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    json!({
        "schema": "cynic.mcp.server_discover.v1",
        "protocol_revision": TARGET_REVISION,
        "method": DISCOVERY_METHOD,
        "server": "cynic",
        "transport_observed": transport,
        "stateless_core": profile.get("stateless_core").cloned().unwrap_or(Value::Null),
        "request_metadata": {
            "mode": profile.get("request_metadata").cloned().unwrap_or(Value::Null),
            "observed": request_meta_observed,
        },
        "tasks": {
            "profile": profile.get("tasks").cloned().unwrap_or(Value::Null),
            "implementation_status": "blocked_scheduler_runtime_unproven",
            "conformance_read_back": false,
        },
        "tool_schema": profile.get("tool_schema").cloned().unwrap_or(Value::Null),
        "wire_requirements": profile.get("wire_requirements").cloned().unwrap_or(Value::Null),
        "tools": tools,
        "implementation_status": implementation_status,
        "read_back": {
            "profile_loaded_from": "infra/agent-exposure.json",
            "discovery_method_served": true,
            "request_metadata_read_back": request_meta_observed,
            "stateless_transport_read_back": transport == "streamable_http_stateless",
            "tasks_extension_conformance_read_back": false,
        },
        "legacy_compatibility": profile.get("legacy_compatibility").cloned().unwrap_or(Value::Null),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_preserves_migration_status_and_proof_dimensions() {
        let exposure = json!({
            "mcp_protocol_profile": {
                "stateless_core": true,
                "request_metadata": "per_request_meta",
                "tasks": "extension",
                "tool_schema": "json_schema_2020_12",
                "implementation_status": "migration_required"
            },
            "mcp_servers": [{"name": "cynic", "tools": [{"name": "cynic_mcp_surface"}]}]
        });

        let result = discovery_result(&exposure, true, "stdio_legacy");

        assert_eq!(result["protocol_revision"], TARGET_REVISION);
        assert_eq!(result["implementation_status"], "migration_required");
        assert_eq!(result["read_back"]["discovery_method_served"], true);
        assert_eq!(result["read_back"]["stateless_transport_read_back"], false);
        assert_eq!(
            result["tasks"]["implementation_status"],
            "blocked_scheduler_runtime_unproven"
        );
        assert_eq!(result["request_metadata"]["observed"], true);
    }

    #[test]
    fn exposed_tools_are_selected_from_the_cynic_server_only() {
        let exposure = json!({
            "mcp_servers": [
                {"name": "other", "tools": [{"name": "ignore"}]},
                {"name": "cynic", "tools": [{"name": "first"}, {"name": "second"}]}
            ]
        });
        assert_eq!(exposed_tool_names(&exposure), vec!["first", "second"]);
    }
}
