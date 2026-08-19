// ── Build runner tools (distributed build trigger + workload observation + forgejo webhook) ──

use crate::types::*;
use crate::types_split::core_params::{BuildTriggerParams, ListWorkloadsParams, ObserveWorkloadParams};

use cynic_host::build_runner_harness::registry::read_registry;
use cynic_host::update_lifecycle::{ArgvRunner, LocalArgvRunner};
use cynic_node::adapters::local_agent_workload_store::LocalAgentWorkloadStore;
use cynic_node::ports::agent_workload_store_port::AgentWorkloadStorePort;
use cynic_node::ports::build_runner::{
    BuildPhase, BuildRunnerPort, BuildTrigger, NetworkMode,
};
use cynic_node::ports::build_runner_adapter::{LocalCommandRunner, SshBuildRunner};
use cynic_protocol::DurableAgentWorkloadRecord;

#[tool_router(router = tool_router_build, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_build_trigger",
        description = "Trigger a distributed build on the network runner (10.10.10.119) for an exact source commit and target phase. Returns the signed pipeline_build_proof bundle. No local cargo, no stack hacks — the runner owns toolchain dispatch."
    )]
    async fn cynic_build_trigger(
        &self,
        params: Parameters<BuildTriggerParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;

        let phase = match p.phase.as_deref() {
            Some("build") => BuildPhase::Build,
            Some("activate") => BuildPhase::Activate,
            Some(other) if other != "check" => {
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&serde_json::json!({
                        "error": format!("invalid phase '{other}' (expected check|build|activate)")
                    }))
                    .unwrap_or_default(),
                )]));
            }
            _ => BuildPhase::Check,
        };
        let network_mode = match p.network_mode.as_deref() {
            Some("online") => NetworkMode::Online,
            _ => NetworkMode::Offline,
        };
        let host = p.host.unwrap_or_else(|| "10.10.10.119".to_string());
        let user = p.user.unwrap_or_else(|| "cynic".to_string());

        let trigger = BuildTrigger {
            source_commit: p.commit,
            phase,
            network_mode,
        };
        let runner = SshBuildRunner {
            host,
            user,
            runner: Box::new(LocalCommandRunner),
        };

        match runner.trigger(&trigger) {
            Ok(proof) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&proof).unwrap_or_default(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&serde_json::json!({
                    "error": format!("{e}")
                }))
                .unwrap_or_default(),
            )])),
        }
    }

    #[tool(
        name = "cynic_build_observe_workload",
        description = "Observe one durable agent workload by id. Returns the workload record from the local store, or a federated read from all registry-declared nodes (merged by LWW: highest committed_at_ms, tie-broken by record_digest). No SPOF — local store works even if kernel/network are down."
    )]
    async fn cynic_build_observe_workload(
        &self,
        params: Parameters<ObserveWorkloadParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;

        let state_root = p.state_root.as_deref().unwrap_or("/var/lib/cynic");
        let registry = p.registry.as_deref().unwrap_or("infra/registry.json");
        let user = p.user.as_deref().unwrap_or("cynic");

        let result = if p.local.unwrap_or(false) {
            observe_workload_local(&p.workload_id, state_root)
        } else {
            observe_workload_federated(&p.workload_id, state_root, registry, user)
        };

        match result {
            Ok(report) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&report).unwrap_or_default(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&serde_json::json!({ "error": format!("{e}") }))
                    .unwrap_or_default(),
            )])),
        }
    }

    #[tool(
        name = "cynic_build_list_workloads",
        description = "List all durable agent workloads. Returns the records from the local store, or a federated read from all registry-declared nodes (merged by LWW: highest committed_at_ms, tie-broken by record_digest). No SPOF — local store works even if kernel/network are down."
    )]
    async fn cynic_build_list_workloads(
        &self,
        params: Parameters<ListWorkloadsParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;

        let state_root = p.state_root.as_deref().unwrap_or("/var/lib/cynic");
        let registry = p.registry.as_deref().unwrap_or("infra/registry.json");
        let user = p.user.as_deref().unwrap_or("cynic");

        let result = if p.local.unwrap_or(false) {
            list_workloads_local(state_root)
        } else {
            list_workloads_federated(state_root, registry, user)
        };

        match result {
            Ok(report) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&report).unwrap_or_default(),
            )])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&serde_json::json!({ "error": format!("{e}") }))
                    .unwrap_or_default(),
            )])),
        }
    }
}

// ── Local store ────────────────────────────────────────────────────────────────

fn observe_workload_local(
    workload_id: &str,
    state_root: &str,
) -> Result<serde_json::Value, String> {
    let store = LocalAgentWorkloadStore::new(
        std::path::PathBuf::from(state_root).join("agent-scheduler/workloads"),
    );
    match store.read(workload_id) {
        Ok(record) => {
            Ok(serde_json::json!({
                "schema": "cynic.mcp.build_observe_workload.v1",
                "status": if record.is_some() { "observed" } else { "unknown" },
                "mutates_runtime": false,
                "workload_id": workload_id,
                "durable_record": record,
                "source": "local",
            }))
        }
        Err(failure) => Err(failure.code),
    }
}

fn list_workloads_local(state_root: &str) -> Result<serde_json::Value, String> {
    let store = LocalAgentWorkloadStore::new(
        std::path::PathBuf::from(state_root).join("agent-scheduler/workloads"),
    );
    match store.durable_records() {
        Ok(records) => {
            Ok(serde_json::json!({
                "schema": "cynic.mcp.build_list_workloads.v1",
                "status": "observed",
                "mutates_runtime": false,
                "count": records.len(),
                "workloads": records,
                "source": "local",
            }))
        }
        Err(failure) => Err(failure.code),
    }
}

// ── Federation via SSH ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct FederatedNode {
    node_id: String,
    ssh_target: String,
}

fn discover_nodes(registry_path: &str, user: &str) -> Result<Vec<FederatedNode>, String> {
    let registry = read_registry(registry_path)?;
    let mut nodes = Vec::new();

    let registry_nodes = registry
        .get("nodes")
        .and_then(|n| n.as_array())
        .ok_or_else(|| "registry: no nodes array".to_string())?;

    for node in registry_nodes {
        let node_id = node
            .get("id")
            .and_then(|s| s.as_str())
            .unwrap_or_default()
            .to_string();
        if node_id.is_empty() {
            continue;
        }

        let ssh_target = node
            .get("transports")
            .and_then(|t| t.get("ssh"))
            .and_then(|s| s.get("target"))
            .and_then(|t| t.as_str())
            .map(|s| format!("{}@{}", user, s))
            .or_else(|| {
                node.get("network")
                    .and_then(|n| n.get("synapse"))
                    .and_then(|s| s.as_str())
                    .map(|ip| format!("{}@{}", user, ip))
            });

        if let Some(target) = ssh_target {
            nodes.push(FederatedNode { node_id, ssh_target });
        }
    }

    Ok(nodes)
}

fn observe_workload_federated(
    workload_id: &str,
    state_root: &str,
    registry_path: &str,
    user: &str,
) -> Result<serde_json::Value, String> {
    let mut merged = std::collections::HashMap::new();
    let local_store = LocalAgentWorkloadStore::new(
        std::path::PathBuf::from(state_root).join("agent-scheduler/workloads"),
    );
    if let Ok(Some(record)) = local_store.read(workload_id) {
        merged.insert(record.record_digest.clone(), record);
    }

    let nodes = discover_nodes(registry_path, user)?;
    let runner = LocalArgvRunner;
    for node in nodes {
        if let Ok(record) = pull_workload(&runner, &node, workload_id, state_root) {
            merged.insert(record.record_digest.clone(), record);
        }
    }

    let result = merge_workloads(merged.into_values().collect());
    Ok(serde_json::json!({
        "schema": "cynic.mcp.build_observe_workload.v1",
        "status": if result.is_some() { "observed" } else { "unknown" },
        "mutates_runtime": false,
        "workload_id": workload_id,
        "durable_record": result,
        "source": "federated",
    }))
}

fn list_workloads_federated(
    state_root: &str,
    registry_path: &str,
    user: &str,
) -> Result<serde_json::Value, String> {
    let mut merged = std::collections::HashMap::new();

    let local_store = LocalAgentWorkloadStore::new(
        std::path::PathBuf::from(state_root).join("agent-scheduler/workloads"),
    );
    if let Ok(records) = local_store.durable_records() {
        for record in records {
            merged.insert(record.record_digest.clone(), record);
        }
    }

    let nodes = discover_nodes(registry_path, user)?;
    let runner = LocalArgvRunner;
    for node in nodes {
        if let Ok(records) = pull_workloads(&runner, &node, state_root) {
            for record in records {
                merged.insert(record.record_digest.clone(), record);
            }
        }
    }

    let all: Vec<DurableAgentWorkloadRecord> = merged.into_values().collect();
    Ok(serde_json::json!({
        "schema": "cynic.mcp.build_list_workloads.v1",
        "status": "observed",
        "mutates_runtime": false,
        "count": all.len(),
        "workloads": all,
        "source": "federated",
    }))
}

fn pull_workload(
    runner: &dyn ArgvRunner,
    node: &FederatedNode,
    workload_id: &str,
    state_root: &str,
) -> Result<DurableAgentWorkloadRecord, String> {
    let argv = vec![
        "ssh",
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=yes",
        "-o",
        "ConnectTimeout=10",
        &node.ssh_target,
        "cynic-node",
        "agent-workload-status",
        "--workload-id",
        workload_id,
        "--state-root",
        state_root,
        "--json",
    ];
    let refs: Vec<&str> = argv.iter().map(|s| *s).collect();
    let output = runner.run(&refs);
    if !output.success() {
        return Err(format!(
            "ssh to {} failed: {}",
            node.node_id,
            output
                .spawn_error
                .unwrap_or_else(|| "non-zero exit".to_string())
        ));
    }
    parse_workload_status(&output.stdout)
        .ok_or_else(|| format!("{}: no workload in response", node.node_id))
}

fn pull_workloads(
    runner: &dyn ArgvRunner,
    node: &FederatedNode,
    state_root: &str,
) -> Result<Vec<DurableAgentWorkloadRecord>, String> {
    let argv = vec![
        "ssh",
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=yes",
        "-o",
        "ConnectTimeout=10",
        &node.ssh_target,
        "cynic-node",
        "agent-workload-list",
        "--state-root",
        state_root,
        "--json",
    ];
    let refs: Vec<&str> = argv.iter().map(|s| *s).collect();
    let output = runner.run(&refs);
    if !output.success() {
        return Err(format!(
            "ssh to {} failed: {}",
            node.node_id,
            output
                .spawn_error
                .unwrap_or_else(|| "non-zero exit".to_string())
        ));
    }
    parse_workload_list(&output.stdout)
}

fn parse_workload_status(raw: &str) -> Option<DurableAgentWorkloadRecord> {
    let value: serde_json::Value = serde_json::from_str(raw.trim()).ok()?;
    let record = value.get("durable_record")?;
    if record.is_null() {
        return None;
    }
    serde_json::from_value(record.clone()).ok()
}

fn parse_workload_list(raw: &str) -> Result<Vec<DurableAgentWorkloadRecord>, String> {
    let value: serde_json::Value = serde_json::from_str(raw.trim())
        .map_err(|e| format!("invalid JSON: {e}"))?;
    let workloads = value
        .get("workloads")
        .and_then(|w| w.as_array())
        .ok_or_else(|| "no workloads array".to_string())?;
    workloads
        .iter()
        .map(|w| serde_json::from_value(w.clone()).map_err(|e| format!("parse workload: {e}")))
        .collect()
}

fn merge_workloads(
    records: Vec<DurableAgentWorkloadRecord>,
) -> Option<DurableAgentWorkloadRecord> {
    if records.is_empty() {
        return None;
    }
    let mut best = &records[0];
    for candidate in &records[1..] {
        if candidate.committed_at_ms > best.committed_at_ms {
            best = candidate;
        } else if candidate.committed_at_ms == best.committed_at_ms
            && candidate.record_digest > best.record_digest
        {
            best = candidate;
        }
    }
    Some(best.clone())
}
