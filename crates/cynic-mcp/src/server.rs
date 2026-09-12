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

    /// Read the placement registry (`infra/registry.json`) and return it as
    /// observed. Desired state, not liveness: node `status` values come from
    /// the file, and only read-back proof promotes them.
    #[tool(
        name = "infra_registry_read",
        description = "Read the placement registry as observed on disk"
    )]
    fn infra_registry_read(&self) -> Result<Json<serde_json::Value>, ErrorData> {
        read_registry().map(Json)
    }

    /// Validate the placement registry structure: schema marker, node
    /// identity/role, service names, and network plane CIDRs. Structural
    /// validation of desired state — not a liveness proof.
    #[tool(
        name = "infra_registry_validate",
        description = "Validate the placement registry structure"
    )]
    fn infra_registry_validate(&self) -> Result<Json<serde_json::Value>, ErrorData> {
        let registry = read_registry()?;
        Ok(Json(validate_registry(&registry)))
    }

    /// Check IP address management coherence from the placement registry:
    /// every assigned address must fall inside its plane CIDR, and no two
    /// owners may claim the same address in the same plane. Pure
    /// reconciliation over observed desired state — no packets sent.
    #[tool(
        name = "network_ipam_check",
        description = "Check IPAM coherence of assigned addresses against plane CIDRs"
    )]
    fn network_ipam_check(&self) -> Result<Json<serde_json::Value>, ErrorData> {
        let registry = read_registry()?;
        Ok(Json(check_ipam(&registry)))
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

fn read_registry() -> Result<serde_json::Value, ErrorData> {
    let root = crate::resolve_workspace_root().map_err(|error| {
        ErrorData::internal_error(format!("workspace root unobserved: {error}"), None)
    })?;
    let content = std::fs::read_to_string(root.join("infra/registry.json")).map_err(|error| {
        ErrorData::internal_error(format!("registry unreadable: {error}"), None)
    })?;
    serde_json::from_str(&content)
        .map_err(|error| ErrorData::internal_error(format!("registry invalid JSON: {error}"), None))
}

fn validate_registry(registry: &serde_json::Value) -> serde_json::Value {
    let mut errors: Vec<String> = Vec::new();
    if registry
        .get("schema")
        .and_then(|schema| schema.as_str())
        .is_none()
    {
        errors.push("missing schema marker".to_string());
    }
    let nodes = registry.get("nodes").and_then(|nodes| nodes.as_array());
    match nodes {
        None => errors.push("missing nodes array".to_string()),
        Some(nodes) => {
            for (index, node) in nodes.iter().enumerate() {
                for field in ["id", "role", "status"] {
                    if node.get(field).and_then(|value| value.as_str()).is_none() {
                        errors.push(format!("nodes[{index}] missing {field}"));
                    }
                }
                if let Some(services) = node
                    .get("services")
                    .and_then(|services| services.as_array())
                {
                    for (service_index, service) in services.iter().enumerate() {
                        if service.get("name").and_then(|name| name.as_str()).is_none() {
                            errors.push(format!(
                                "nodes[{index}].services[{service_index}] missing name"
                            ));
                        }
                    }
                }
            }
        }
    }
    if let Some(planes) = registry
        .get("network_policy")
        .and_then(|policy| policy.get("planes"))
        .and_then(|planes| planes.as_object())
    {
        for (plane, descriptor) in planes {
            match descriptor.get("cidr").and_then(|cidr| cidr.as_str()) {
                None => errors.push(format!("network_policy.planes.{plane} missing cidr")),
                Some(cidr) => {
                    if parse_cidr(cidr).is_none() {
                        errors.push(format!("network_policy.planes.{plane} invalid cidr {cidr}"));
                    }
                }
            }
        }
    }
    serde_json::json!({
        "valid": errors.is_empty(),
        "error_count": errors.len(),
        "errors": errors,
    })
}

/// One observed address assignment: which owner claims which IP in which plane.
#[derive(Debug, Clone, PartialEq, Eq)]
struct IpAssignment {
    owner: String,
    plane: String,
    ip: String,
}

fn parse_ipv4(text: &str) -> Option<u32> {
    let mut address: u32 = 0;
    let mut parts = 0;
    for part in text.split('.') {
        let octet: u32 = part.parse().ok()?;
        if octet > 255 || parts == 4 {
            return None;
        }
        address = (address << 8) | octet;
        parts += 1;
    }
    (parts == 4).then_some(address)
}

fn parse_cidr(text: &str) -> Option<(u32, u32)> {
    let (base, prefix) = text.split_once('/')?;
    let prefix: u32 = prefix.parse().ok()?;
    if prefix > 32 {
        return None;
    }
    let network = parse_ipv4(base)?;
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    Some((network & mask, mask))
}

fn collect_assignments(registry: &serde_json::Value) -> Vec<IpAssignment> {
    let mut assignments = Vec::new();
    if let Some(reservations) = registry
        .get("network_policy")
        .and_then(|policy| policy.get("reservations"))
        .and_then(|reservations| reservations.as_array())
    {
        for reservation in reservations {
            let owner = reservation
                .get("name")
                .and_then(|name| name.as_str())
                .unwrap_or("unnamed_reservation");
            for plane in ["management", "synapse"] {
                if let Some(ip) = reservation.get(plane).and_then(|ip| ip.as_str()) {
                    assignments.push(IpAssignment {
                        owner: owner.to_string(),
                        plane: plane.to_string(),
                        ip: ip.to_string(),
                    });
                }
            }
        }
    }
    if let Some(nodes) = registry.get("nodes").and_then(|nodes| nodes.as_array()) {
        for node in nodes {
            let owner = node
                .get("id")
                .and_then(|id| id.as_str())
                .unwrap_or("unnamed_node");
            if let Some(network) = node.get("network").and_then(|network| network.as_object()) {
                for (plane, ip) in network {
                    if let Some(ip) = ip.as_str() {
                        assignments.push(IpAssignment {
                            owner: owner.to_string(),
                            plane: plane.clone(),
                            ip: ip.to_string(),
                        });
                    }
                }
            }
        }
    }
    assignments
}

fn check_ipam(registry: &serde_json::Value) -> serde_json::Value {
    let mut cidrs: std::collections::BTreeMap<String, (u32, u32)> =
        std::collections::BTreeMap::new();
    if let Some(planes) = registry
        .get("network_policy")
        .and_then(|policy| policy.get("planes"))
        .and_then(|planes| planes.as_object())
    {
        for (plane, descriptor) in planes {
            if let Some(cidr) = descriptor
                .get("cidr")
                .and_then(|cidr| cidr.as_str())
                .and_then(parse_cidr)
            {
                cidrs.insert(plane.clone(), cidr);
            }
        }
    }
    let assignments = collect_assignments(registry);
    let mut conflicts: Vec<String> = Vec::new();
    let mut out_of_range: Vec<String> = Vec::new();
    let mut unknown_plane: Vec<String> = Vec::new();
    let mut seen: std::collections::BTreeMap<(String, String), String> =
        std::collections::BTreeMap::new();
    for assignment in &assignments {
        let address = parse_ipv4(&assignment.ip);
        match address {
            None => out_of_range.push(format!(
                "{} claims unparsable IP {} in plane {}",
                assignment.owner, assignment.ip, assignment.plane
            )),
            Some(address) => {
                match cidrs.get(&assignment.plane) {
                    None => unknown_plane.push(format!(
                        "{} claims {} in plane {} without a declared CIDR",
                        assignment.owner, assignment.ip, assignment.plane
                    )),
                    Some((network, mask)) => {
                        if address & mask != *network {
                            out_of_range.push(format!(
                                "{} claims {} outside plane {} range",
                                assignment.owner, assignment.ip, assignment.plane
                            ));
                        }
                    }
                }
                let key = (assignment.plane.clone(), assignment.ip.clone());
                if let Some(first) = seen.get(&key) {
                    if first != &assignment.owner {
                        conflicts.push(format!(
                            "{} and {} both claim {} in plane {}",
                            first, assignment.owner, assignment.ip, assignment.plane
                        ));
                    }
                } else {
                    seen.insert(key, assignment.owner.clone());
                }
            }
        }
    }
    serde_json::json!({
        "assignment_count": assignments.len(),
        "conflicts": conflicts,
        "out_of_range": out_of_range,
        "unknown_plane": unknown_plane,
        "coherent": conflicts.is_empty() && out_of_range.is_empty() && unknown_plane.is_empty(),
    })
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

    fn fixture_registry() -> serde_json::Value {
        serde_json::json!({
            "schema": "cynic.registry.v1.minimal",
            "network_policy": {
                "planes": {
                    "management": { "cidr": "192.168.100.0/24" },
                    "synapse": { "cidr": "10.10.10.0/24" }
                },
                "reservations": [
                    { "name": "a", "management": "192.168.100.1", "synapse": "10.10.10.1" },
                    { "name": "b", "management": "192.168.100.2" }
                ]
            },
            "nodes": [
                { "id": "a", "role": "hypervisor", "status": "observed",
                  "network": { "management": "192.168.100.1", "synapse": "10.10.10.1" } },
                { "id": "b", "role": "builder", "status": "candidate",
                  "services": [ { "name": "cynic-node" } ] }
            ]
        })
    }

    #[test]
    fn validate_registry_accepts_fixture() {
        let report = validate_registry(&fixture_registry());
        assert_eq!(report.get("valid"), Some(&serde_json::Value::Bool(true)));
    }

    #[test]
    fn validate_registry_reports_missing_schema() {
        let report = validate_registry(&serde_json::json!({ "nodes": [] }));
        assert_eq!(report.get("valid"), Some(&serde_json::Value::Bool(false)));
    }

    #[test]
    fn ipam_accepts_coherent_fixture() {
        let report = check_ipam(&fixture_registry());
        assert_eq!(report.get("coherent"), Some(&serde_json::Value::Bool(true)));
    }

    #[test]
    fn ipam_detects_duplicate_and_out_of_range() {
        let mut registry = fixture_registry();
        registry["network_policy"]["reservations"]
            .as_array_mut()
            .expect("reservations")
            .push(serde_json::json!({ "name": "c", "management": "192.168.100.2" }));
        registry["nodes"]
            .as_array_mut()
            .expect("nodes")
            .push(serde_json::json!({
                "id": "rogue", "role": "x", "status": "candidate",
                "network": { "synapse": "11.0.0.9" }
            }));
        let report = check_ipam(&registry);
        assert_eq!(
            report.get("coherent"),
            Some(&serde_json::Value::Bool(false))
        );
        let conflicts = report
            .get("conflicts")
            .and_then(|conflicts| conflicts.as_array())
            .expect("conflicts");
        assert!(conflicts.iter().any(|conflict| {
            conflict
                .as_str()
                .is_some_and(|text| text.contains("192.168.100.2"))
        }));
        let out_of_range = report
            .get("out_of_range")
            .and_then(|items| items.as_array())
            .expect("out_of_range");
        assert!(
            out_of_range
                .iter()
                .any(|item| item.as_str().is_some_and(|text| text.contains("11.0.0.9")))
        );
    }

    #[test]
    fn ipam_separates_unknown_plane_from_out_of_range() {
        let mut registry = fixture_registry();
        registry["nodes"]
            .as_array_mut()
            .expect("nodes")
            .push(serde_json::json!({
                "id": "forgejo-like", "role": "x", "status": "candidate",
                "network": { "local": "10.10.10.101" }
            }));
        let report = check_ipam(&registry);
        let unknown_plane = report
            .get("unknown_plane")
            .and_then(|items| items.as_array())
            .expect("unknown_plane");
        assert!(
            unknown_plane
                .iter()
                .any(|item| item.as_str().is_some_and(|text| text.contains("local")))
        );
        let out_of_range = report
            .get("out_of_range")
            .and_then(|items| items.as_array())
            .expect("out_of_range");
        assert!(out_of_range.is_empty());
    }
}
