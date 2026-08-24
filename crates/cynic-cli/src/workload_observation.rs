//! Workload observation surfaces (CLI + MCP projection target).
//!
//! Provides read-only observation of durable agent workloads across the CYNIC
//! federation. Two modes:
//!   - `--local`: scan the node-local `LocalAgentWorkloadStore` only.
//!   - default: pull from every node declared in the registry via SSH,
//!     merge with LWW (committed_at_ms, tie-breaker record_digest).
//!
//! No SPOF: local store works even if the kernel and network are down.
//! Federation works even if some nodes are unreachable.

use std::collections::HashMap;
use std::path::PathBuf;

use cynic_host::build_runner_harness::registry::read_registry;
use cynic_host::update_lifecycle::{ArgvRunner, LocalArgvRunner};
use cynic_node::adapters::local_agent_workload_store::LocalAgentWorkloadStore;
use cynic_node::ports::agent_workload_store_port::AgentWorkloadStorePort;
use cynic_protocol::DurableAgentWorkloadRecord;

/// Observe one workload by id.
pub(crate) fn execute_observe_workload(
    workload_id: &str,
    state_root: &str,
    local: bool,
    registry_path: &str,
    user: &str,
    json: bool,
) -> Result<(), String> {
    if local {
        return observe_workload_local(workload_id, state_root, json);
    }
    observe_workload_federated(workload_id, state_root, registry_path, user, json)
}

/// List all durable workloads.
pub(crate) fn execute_list_workloads(
    state_root: &str,
    local: bool,
    registry_path: &str,
    user: &str,
    json: bool,
) -> Result<(), String> {
    if local {
        return list_workloads_local(state_root, json);
    }
    list_workloads_federated(state_root, registry_path, user, json)
}

// ── Local store ────────────────────────────────────────────────────────────────

fn observe_workload_local(workload_id: &str, state_root: &str, json: bool) -> Result<(), String> {
    let store =
        LocalAgentWorkloadStore::new(PathBuf::from(state_root).join("agent-scheduler/workloads"));
    match store.read(workload_id) {
        Ok(record) => {
            let report = serde_json::json!({
                "schema": "cynic.cli.build_observe_workload.v1",
                "status": if record.is_some() { "observed" } else { "unknown" },
                "mutates_runtime": false,
                "workload_id": workload_id,
                "durable_record": record,
                "source": "local",
            });
            emit(report, json);
            Ok(())
        }
        Err(failure) => Err(failure.code),
    }
}

fn list_workloads_local(state_root: &str, json: bool) -> Result<(), String> {
    let store =
        LocalAgentWorkloadStore::new(PathBuf::from(state_root).join("agent-scheduler/workloads"));
    match store.durable_records() {
        Ok(records) => {
            let report = serde_json::json!({
                "schema": "cynic.cli.build_list_workloads.v1",
                "status": "observed",
                "mutates_runtime": false,
                "count": records.len(),
                "workloads": records,
                "source": "local",
            });
            emit(report, json);
            Ok(())
        }
        Err(failure) => Err(failure.code),
    }
}

// ── Federation via SSH ─────────────────────────────────────────────────────────

fn observe_workload_federated(
    workload_id: &str,
    state_root: &str,
    registry_path: &str,
    user: &str,
    json: bool,
) -> Result<(), String> {
    // Start with local record
    let mut merged = HashMap::new();
    let local_store =
        LocalAgentWorkloadStore::new(PathBuf::from(state_root).join("agent-scheduler/workloads"));
    if let Ok(Some(record)) = local_store.read(workload_id) {
        merged.insert(record.record_digest.clone(), record);
    }

    // Pull from each node in the registry
    let nodes = discover_nodes(registry_path, user)?;
    let runner = LocalArgvRunner;
    for node in nodes {
        if let Ok(record) = pull_workload(&runner, &node, workload_id, state_root) {
            merged.insert(record.record_digest.clone(), record);
        }
    }

    // Merge: pick the one with the highest committed_at_ms
    let result = merge_workloads(merged.into_values().collect());
    let report = serde_json::json!({
        "schema": "cynic.cli.build_observe_workload.v1",
        "status": if result.is_some() { "observed" } else { "unknown" },
        "mutates_runtime": false,
        "workload_id": workload_id,
        "durable_record": result,
        "source": "federated",
    });
    emit(report, json);
    Ok(())
}

fn list_workloads_federated(
    state_root: &str,
    registry_path: &str,
    user: &str,
    json: bool,
) -> Result<(), String> {
    let mut merged = HashMap::new();

    // Local records
    let local_store =
        LocalAgentWorkloadStore::new(PathBuf::from(state_root).join("agent-scheduler/workloads"));
    if let Ok(records) = local_store.durable_records() {
        for record in records {
            merged.insert(record.record_digest.clone(), record);
        }
    }

    // Pull from each node in the registry
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
    let report = serde_json::json!({
        "schema": "cynic.cli.build_list_workloads.v1",
        "status": "observed",
        "mutates_runtime": false,
        "count": all.len(),
        "workloads": all,
        "source": "federated",
    });
    emit(report, json);
    Ok(())
}

// ── Node discovery ─────────────────────────────────────────────────────────────

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

        // Extract SSH target from transports.ssh.target or network.synapse
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

        nodes.push(FederatedNode {
            node_id,
            ssh_target: ssh_target.unwrap_or_default(),
        });
    }

    Ok(nodes)
}

// ── Pull via SSH ──────────────────────────────────────────────────────────────

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
    let value: serde_json::Value =
        serde_json::from_str(raw.trim()).map_err(|e| format!("invalid JSON: {e}"))?;
    let workloads = value
        .get("workloads")
        .and_then(|w| w.as_array())
        .ok_or_else(|| "no workloads array".to_string())?;
    workloads
        .iter()
        .map(|w| serde_json::from_value(w.clone()).map_err(|e| format!("parse workload: {e}")))
        .collect()
}

// ── Merge strategy: LWW with deterministic tie-breaker ────────────────────────

fn merge_workloads(records: Vec<DurableAgentWorkloadRecord>) -> Option<DurableAgentWorkloadRecord> {
    if records.is_empty() {
        return None;
    }
    let mut best = &records[0];
    for candidate in &records[1..] {
        if candidate.committed_at_ms > best.committed_at_ms {
            best = candidate;
        } else if candidate.committed_at_ms == best.committed_at_ms {
            // Deterministic tie-breaker: higher record_digest wins
            if candidate.record_digest > best.record_digest {
                best = candidate;
            }
        }
    }
    Some(best.clone())
}

// ── Output ─────────────────────────────────────────────────────────────────────

fn emit(value: serde_json::Value, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&value).unwrap_or_default()
        );
    } else {
        let status = value
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("unknown");
        if let Some(id) = value.get("workload_id").and_then(|s| s.as_str()) {
            println!("workload {id} -> {status}");
        } else if let Some(count) = value.get("count").and_then(|c| c.as_u64()) {
            println!("{count} workloads -> {status}");
        } else {
            println!("{status}");
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeArgvRunner {
        response: String,
    }

    impl ArgvRunner for FakeArgvRunner {
        fn run(&self, _argv: &[&str]) -> ArgvOutput {
            ArgvOutput {
                exit_code: Some(0),
                stdout: self.response.clone(),
                stderr: String::new(),
                spawn_error: None,
            }
        }
    }

    #[test]
    fn merge_lww_picks_most_recent() {
        let older = DurableAgentWorkloadRecord {
            schema: "test".to_string(),
            revision: 1,
            previous_record_digest: None,
            record_digest: "aaa".to_string(),
            record: cynic_protocol::AgentWorkloadRecord {
                schema: "test".to_string(),
                envelope: cynic_protocol::AgentWorkloadEnvelope {
                    schema: "test".to_string(),
                    workload_id: "w1".to_string(),
                    idempotency_key: "k1".to_string(),
                    delegation: cynic_protocol::AgentDelegationEnvelope {
                        schema: "test".to_string(),
                        delegation_id: "d1".to_string(),
                        node_id: "n1".to_string(),
                        agent_id: "a1".to_string(),
                        parent_agent_id: None,
                        depth: 0,
                        objective: "build".to_string(),
                        authority_snapshot: cynic_protocol::AgentEvidenceRef {
                            schema: "test".to_string(),
                            subject: "s1".to_string(),
                            digest: "dig1".to_string(),
                            lineage: None,
                        },
                        context_refs: vec![],
                        capabilities: vec![],
                        budget: cynic_protocol::AgentBudget {
                            context_tokens: 0,
                            actions: 0,
                            children: 0,
                            wall_time_ms: 0,
                        },
                        expected_proofs: vec![],
                    },
                    policy_admission: cynic_protocol::PolicyAdmissionReport {
                        schema: "test".to_string(),
                        intent: cynic_protocol::PolicyIntentRef {
                            schema: "test".to_string(),
                            subject: "s1".to_string(),
                            digest: "dig1".to_string(),
                            action: "check".to_string(),
                            role: "build_runner".to_string(),
                            mutation_authorized: true,
                        },
                        decision: cynic_protocol::PolicyDecision::Allow,
                        evidence: serde_json::json!({}),
                        evidence_refs: vec![],
                        status: "allowed".to_string(),
                        mutates_host_network: false,
                        capability_surface: vec![],
                        residual_unknowns: vec![],
                        node_capabilities: vec![],
                        admission_read_back: None,
                        admission_read_back_verified: false,
                        admission_read_back_violations: vec![],
                    },
                    action: cynic_protocol::ExecutionAuthorization {
                        schema: "test".to_string(),
                        input_digest: None,
                        input_schema: "test".to_string(),
                        input_kind: "test".to_string(),
                        payload: serde_json::json!({}),
                    },
                    input: None,
                    expected_proofs: vec![],
                    mutation_contract: cynic_protocol::MutationContract {
                        schema: "test".to_string(),
                        mutates_host_network: false,
                        requires_privilege: false,
                        capability_surface: vec![],
                    },
                    scope: cynic_protocol::PolicyScope {
                        schema: "test".to_string(),
                        node_id: "n1".to_string(),
                        resource: "build".to_string(),
                        action: "check".to_string(),
                        role: "build_runner".to_string(),
                    },
                    policy_set: cynic_protocol::PolicySetRef {
                        schema: "test".to_string(),
                        subject: "s1".to_string(),
                        digest: "dig1".to_string(),
                    },
                    residual_unknowns: vec![],
                    build_grant: None,
                },
                state: cynic_protocol::AgentWorkloadState::Dispatched,
                claim: None,
                last_fencing_token: 0,
                cancellation_generation: 0,
                consumed_actions: 0,
                completion: None,
            },
            committed_at_ms: 1000,
        };
        let newer = DurableAgentWorkloadRecord {
            committed_at_ms: 2000,
            record_digest: "bbb".to_string(),
            ..older.clone()
        };

        let result = merge_workloads(vec![older, newer]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().committed_at_ms, 2000);
    }

    #[test]
    fn merge_tie_breaker_deterministic() {
        let a = DurableAgentWorkloadRecord {
            committed_at_ms: 1000,
            record_digest: "aaa".to_string(),
            ..dummy_record()
        };
        let b = DurableAgentWorkloadRecord {
            committed_at_ms: 1000,
            record_digest: "bbb".to_string(),
            ..dummy_record()
        };

        let result = merge_workloads(vec![a.clone(), b.clone()]);
        assert!(result.is_some());
        // Higher digest wins on tie
        assert_eq!(result.unwrap().record_digest, "bbb");
    }

    #[test]
    fn merge_empty_returns_none() {
        assert!(merge_workloads(vec![]).is_none());
    }

    fn dummy_record() -> DurableAgentWorkloadRecord {
        DurableAgentWorkloadRecord {
            schema: "test".to_string(),
            revision: 1,
            previous_record_digest: None,
            record_digest: "dummy".to_string(),
            record: cynic_protocol::AgentWorkloadRecord {
                schema: "test".to_string(),
                envelope: cynic_protocol::AgentWorkloadEnvelope {
                    schema: "test".to_string(),
                    workload_id: "w1".to_string(),
                    idempotency_key: "k1".to_string(),
                    delegation: cynic_protocol::AgentDelegationEnvelope {
                        schema: "test".to_string(),
                        delegation_id: "d1".to_string(),
                        node_id: "n1".to_string(),
                        agent_id: "a1".to_string(),
                        parent_agent_id: None,
                        depth: 0,
                        objective: "build".to_string(),
                        authority_snapshot: cynic_protocol::AgentEvidenceRef {
                            schema: "test".to_string(),
                            subject: "s1".to_string(),
                            digest: "dig1".to_string(),
                            lineage: None,
                        },
                        context_refs: vec![],
                        capabilities: vec![],
                        budget: cynic_protocol::AgentBudget {
                            context_tokens: 0,
                            actions: 0,
                            children: 0,
                            wall_time_ms: 0,
                        },
                        expected_proofs: vec![],
                    },
                    policy_admission: cynic_protocol::PolicyAdmissionReport {
                        schema: "test".to_string(),
                        intent: cynic_protocol::PolicyIntentRef {
                            schema: "test".to_string(),
                            subject: "s1".to_string(),
                            digest: "dig1".to_string(),
                            action: "check".to_string(),
                            role: "build_runner".to_string(),
                            mutation_authorized: true,
                        },
                        decision: cynic_protocol::PolicyDecision::Allow,
                        evidence: serde_json::json!({}),
                        evidence_refs: vec![],
                        status: "allowed".to_string(),
                        mutates_host_network: false,
                        capability_surface: vec![],
                        residual_unknowns: vec![],
                        node_capabilities: vec![],
                        admission_read_back: None,
                        admission_read_back_verified: false,
                        admission_read_back_violations: vec![],
                    },
                    action: cynic_protocol::ExecutionAuthorization {
                        schema: "test".to_string(),
                        input_digest: None,
                        input_schema: "test".to_string(),
                        input_kind: "test".to_string(),
                        payload: serde_json::json!({}),
                    },
                    input: None,
                    expected_proofs: vec![],
                    mutation_contract: cynic_protocol::MutationContract {
                        schema: "test".to_string(),
                        mutates_host_network: false,
                        requires_privilege: false,
                        capability_surface: vec![],
                    },
                    scope: cynic_protocol::PolicyScope {
                        schema: "test".to_string(),
                        node_id: "n1".to_string(),
                        resource: "build".to_string(),
                        action: "check".to_string(),
                        role: "build_runner".to_string(),
                    },
                    policy_set: cynic_protocol::PolicySetRef {
                        schema: "test".to_string(),
                        subject: "s1".to_string(),
                        digest: "dig1".to_string(),
                    },
                    residual_unknowns: vec![],
                    build_grant: None,
                },
                state: cynic_protocol::AgentWorkloadState::Dispatched,
                claim: None,
                last_fencing_token: 0,
                cancellation_generation: 0,
                consumed_actions: 0,
                completion: None,
            },
            committed_at_ms: 1000,
        }
    }

    #[test]
    fn parse_workload_status_extracts_record() {
        let raw = serde_json::json!({
            "schema": "cynic.node.agent_workload_status.v1",
            "status": "observed",
            "mutates_runtime": false,
            "workload_id": "w1",
            "durable_record": dummy_record(),
        });
        let result = parse_workload_status(&raw.to_string());
        assert!(result.is_some());
        assert_eq!(result.unwrap().record_digest, "dummy");
    }

    #[test]
    fn parse_workload_list_extracts_array() {
        let raw = serde_json::json!({
            "schema": "cynic.node.agent_workload_list.v1",
            "status": "observed",
            "count": 2,
            "workloads": [dummy_record(), dummy_record()],
        });
        let result = parse_workload_list(&raw.to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }
}
