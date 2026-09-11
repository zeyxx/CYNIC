//! MCP tool implementations. Each `#[tool]` below is a real executor with
//! live read-back: it observes the substrate and returns evidence, or a
//! typed error. No canned successes.
//!
//! Slice 1 is read-only and in-process. Every tool added later registers the
//! same way: the tool attribute macro on a method inside the tool_router
//! impl block.

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{ErrorData, schemars, tool, tool_router};
use serde::{Deserialize, Serialize};

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

    /// Inventory the machine-readable authority contracts
    /// (`contracts/foundation.json`, `infra/runtime-authority.json`,
    /// `infra/registry.json`, `infra/agent-exposure.json`): schema, status,
    /// and authority order as observed on disk. A missing or unparsable
    /// file is reported per file — never a silent pass.
    #[tool(
        name = "cynic_api_contracts",
        description = "Inventory the machine-readable authority contracts as observed on disk"
    )]
    fn cynic_api_contracts(&self) -> Json<serde_json::Value> {
        Json(api_contracts_inventory())
    }

    /// Decide harness progression with the pure host reconciliation:
    /// can the system move from `current_kind` to `target_kind` given the
    /// satisfied and unknown proof categories? Unknown kind or category
    /// names are typed errors, never guesses.
    #[tool(
        name = "cynic_harness_progression",
        description = "Decide runtime harness progression from current kind, target kind, and proof categories"
    )]
    fn cynic_harness_progression(
        &self,
        Parameters(args): Parameters<HarnessProgressionArgs>,
    ) -> Result<Json<serde_json::Value>, ErrorData> {
        decide_harness_progression(&args).map(Json)
    }

    /// Observe a runtime binary's MCP surface by executing
    /// `<binary> --contract-json` and reading back the compatibility plan.
    /// Read-only observation: the binary runs, nothing is mutated.
    #[tool(
        name = "cynic_agent_runtime_observe",
        description = "Observe a runtime binary MCP surface via --contract-json read-back"
    )]
    fn cynic_agent_runtime_observe(
        &self,
        Parameters(args): Parameters<RuntimeObserveArgs>,
    ) -> Result<Json<serde_json::Value>, ErrorData> {
        if args.binary.trim().is_empty() {
            return Err(ErrorData::invalid_params(
                "binary must name an executable to observe",
                None,
            ));
        }
        let observation = cynic_host::mcp_runtime::observe_mcp_runtime(
            args.binary.trim(),
            &cynic_host::update_lifecycle::LocalArgvRunner,
        );
        serde_json::to_value(&observation)
            .map(Json)
            .map_err(|error| ErrorData::internal_error(format!("serialize failed: {error}"), None))
    }
}

/// Contract files inventoried by `cynic_api_contracts`, relative to the
/// workspace root. Desired-state authorities — the tool only observes them.
const CONTRACT_FILES: &[&str] = &[
    "contracts/foundation.json",
    "infra/runtime-authority.json",
    "infra/registry.json",
    "infra/agent-exposure.json",
];

/// Arguments for `cynic_harness_progression`. Kind and category names use the
/// host vocabulary (`ClosedLocalLab`, `Bootstrap`, ...); unknown names fail.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HarnessProgressionArgs {
    /// Current harness kind, e.g. `ClosedLocalLab`.
    pub current_kind: String,
    /// Target harness kind, e.g. `Conformance`.
    pub target_kind: String,
    /// Satisfied proof categories, e.g. `["Bootstrap", "Network"]`.
    #[serde(default)]
    pub satisfied: Vec<String>,
    /// Unobservable proof categories, e.g. `["Hardware"]`.
    #[serde(default)]
    pub unknown: Vec<String>,
}

/// Arguments for `cynic_agent_runtime_observe`.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RuntimeObserveArgs {
    /// Binary to observe (resolved via PATH or explicit path).
    pub binary: String,
}

fn api_contracts_inventory() -> serde_json::Value {
    let root = crate::resolve_workspace_root().unwrap_or_else(|_| ".".into());
    let files: Vec<serde_json::Value> = CONTRACT_FILES
        .iter()
        .map(|relative| {
            let path = root.join(relative);
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => serde_json::json!({
                        "path": relative,
                        "readable": true,
                        "valid_json": true,
                        "schema": value.get("schema"),
                        "status": value.get("status"),
                    }),
                    Err(error) => serde_json::json!({
                        "path": relative,
                        "readable": true,
                        "valid_json": false,
                        "error": error.to_string(),
                    }),
                },
                Err(error) => serde_json::json!({
                    "path": relative,
                    "readable": false,
                    "valid_json": false,
                    "error": error.to_string(),
                }),
            }
        })
        .collect();
    serde_json::json!({ "contracts": files })
}

fn parse_harness_kind(
    name: &str,
) -> Result<cynic_host::runtime_harness::RuntimeHarnessKind, ErrorData> {
    use cynic_host::runtime_harness::RuntimeHarnessKind as Kind;
    match name {
        "ClosedLocalLab" => Ok(Kind::ClosedLocalLab),
        "Conformance" => Ok(Kind::Conformance),
        "Staging" => Ok(Kind::Staging),
        "Canary" => Ok(Kind::Canary),
        "Production" => Ok(Kind::Production),
        _ => Err(ErrorData::invalid_params(
            format!("unknown harness kind: {name}"),
            None,
        )),
    }
}

fn parse_proof_category(
    name: &str,
) -> Result<cynic_host::harness_progression::ProofCategory, ErrorData> {
    use cynic_host::harness_progression::ProofCategory as Category;
    match name {
        "Bootstrap" => Ok(Category::Bootstrap),
        "Hardware" => Ok(Category::Hardware),
        "HostConformance" => Ok(Category::HostConformance),
        "BuildReproduction" => Ok(Category::BuildReproduction),
        "Network" => Ok(Category::Network),
        "TargetAttestation" => Ok(Category::TargetAttestation),
        "RollbackIntent" => Ok(Category::RollbackIntent),
        _ => Err(ErrorData::invalid_params(
            format!("unknown proof category: {name}"),
            None,
        )),
    }
}

fn decide_harness_progression(
    args: &HarnessProgressionArgs,
) -> Result<serde_json::Value, ErrorData> {
    let current_kind = parse_harness_kind(&args.current_kind)?;
    let target_kind = parse_harness_kind(&args.target_kind)?;
    let satisfied = args
        .satisfied
        .iter()
        .map(|name| parse_proof_category(name))
        .collect::<Result<Vec<_>, _>>()?;
    let unknown = args
        .unknown
        .iter()
        .map(|name| parse_proof_category(name))
        .collect::<Result<Vec<_>, _>>()?;
    let evidence = cynic_host::harness_progression::HarnessProgressionEvidence {
        current_kind,
        target_kind,
        satisfied,
        pending: vec![],
        unknown,
    };
    let verdict = cynic_host::harness_progression::decide_harness_progression(&evidence);
    serde_json::to_value(&verdict)
        .map_err(|error| ErrorData::internal_error(format!("serialize failed: {error}"), None))
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

    #[test]
    fn contracts_inventory_reports_four_files() {
        let server = CynicMcpServer;
        let value = (server.cynic_api_contracts()).0;
        let contracts = value
            .get("contracts")
            .and_then(|contracts| contracts.as_array())
            .expect("contracts array");
        assert_eq!(contracts.len(), 4);
        assert!(
            contracts
                .iter()
                .all(|entry| entry.get("path").and_then(|path| path.as_str()).is_some())
        );
    }

    #[test]
    fn harness_progression_rejects_unknown_kinds() {
        let args = HarnessProgressionArgs {
            current_kind: "Nope".to_string(),
            target_kind: "Conformance".to_string(),
            satisfied: vec![],
            unknown: vec![],
        };
        assert!(decide_harness_progression(&args).is_err());
    }

    #[test]
    fn harness_progression_blocks_skipped_steps() {
        // Lab -> Staging skips Conformance: must block, whatever the proofs.
        let args = HarnessProgressionArgs {
            current_kind: "ClosedLocalLab".to_string(),
            target_kind: "Staging".to_string(),
            satisfied: vec!["Bootstrap".to_string()],
            unknown: vec![],
        };
        let verdict = decide_harness_progression(&args).expect("verdict");
        let text = verdict.to_string();
        assert!(text.contains("Blocked") || text.contains("blocked"));
    }

    #[test]
    fn runtime_observe_rejects_empty_binary() {
        let server = CynicMcpServer;
        let args = RuntimeObserveArgs {
            binary: "   ".to_string(),
        };
        assert!(
            server
                .cynic_agent_runtime_observe(Parameters(args))
                .is_err()
        );
    }
}
