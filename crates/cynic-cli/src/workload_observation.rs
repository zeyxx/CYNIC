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

fn observe_workload_local(workload_id: &str, state_root: &str, json: bool) -> Result<(), String> {
    let store = LocalAgentWorkloadStore::new(PathBuf::from(state_root));
    let record = store
        .read(workload_id)
        .map_err(|e| format!("local store read error: {e:?}"))?;

    match record {
        Some(durable) => {
            emit_single_record(&durable, json);
            Ok(())
        }
        None => Err(format!("workload not found in local store: {workload_id}")),
    }
}

fn list_workloads_local(state_root: &str, json: bool) -> Result<(), String> {
    let store = LocalAgentWorkloadStore::new(PathBuf::from(state_root));
    let records = store
        .durable_records()
        .map_err(|e| format!("local store list error: {e:?}"))?;

    emit_record_list(&records, json);
    Ok(())
}

fn observe_workload_federated(
    workload_id: &str,
    state_root: &str,
    registry_path: &str,
    user: &str,
    json: bool,
) -> Result<(), String> {
    let nodes = resolve_target_nodes(registry_path)?;
    let runner = LocalArgvRunner;
    let mut candidate_records: Vec<DurableAgentWorkloadRecord> = Vec::new();

    // 1. Local read first (zero-network fast path)
    let local_store = LocalAgentWorkloadStore::new(PathBuf::from(state_root));
    if let Ok(Some(local_rec)) = local_store.read(workload_id) {
        candidate_records.push(local_rec);
    }

    // 2. Remote reads via SSH in parallel / sequential loop
    for node in &nodes {
        let cmd = format!(
            "cynic node agent-workload observe --workload-id {} --local --json",
            workload_id
        );
        let ssh_args = [
            "ssh",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=3",
            &format!("{}@{}", user, node),
            &cmd,
        ];
        let out = runner.run(&ssh_args);
        if out.exit_code == Some(0) {
            if let Some(record) = parse_workload_status(&out.stdout) {
                candidate_records.push(record);
            }
        }
    }

    // 3. Reconcile LWW
    let winning = merge_workloads(candidate_records);
    match winning {
        Some(record) => {
            emit_single_record(&record, json);
            Ok(())
        }
        None => Err(format!(
            "workload not found across federation: {workload_id}"
        )),
    }
}

fn list_workloads_federated(
    state_root: &str,
    registry_path: &str,
    user: &str,
    json: bool,
) -> Result<(), String> {
    let nodes = resolve_target_nodes(registry_path)?;
    let runner = LocalArgvRunner;
    let mut by_workload_id: HashMap<String, Vec<DurableAgentWorkloadRecord>> = HashMap::new();

    // 1. Local list
    let local_store = LocalAgentWorkloadStore::new(PathBuf::from(state_root));
    if let Ok(local_records) = local_store.durable_records() {
        for rec in local_records {
            by_workload_id
                .entry(rec.record.envelope.workload_id.clone())
                .or_default()
                .push(rec);
        }
    }

    // 2. Remote lists
    for node in &nodes {
        let cmd = "cynic node agent-workload list --local --json";
        let ssh_args = [
            "ssh",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=3",
            &format!("{}@{}", user, node),
            cmd,
        ];
        let out = runner.run(&ssh_args);
        if out.exit_code == Some(0) {
            if let Ok(records) = parse_workload_list(&out.stdout) {
                for rec in records {
                    by_workload_id
                        .entry(rec.record.envelope.workload_id.clone())
                        .or_default()
                        .push(rec);
                }
            }
        }
    }

    // 3. Merge each workload by LWW
    let mut merged: Vec<DurableAgentWorkloadRecord> = by_workload_id
        .into_values()
        .filter_map(merge_workloads)
        .collect();

    // Sort deterministically by committed_at_ms descending
    merged.sort_by(|a, b| b.committed_at_ms.cmp(&a.committed_at_ms));

    emit_record_list(&merged, json);
    Ok(())
}

fn resolve_target_nodes(registry_path: &str) -> Result<Vec<String>, String> {
    let reg = read_registry(registry_path).map_err(|e| format!("failed to read registry: {e}"))?;

    let nodes = reg
        .get("nodes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "registry_nodes_must_be_array".to_string())?;

    let result: Vec<String> = nodes
        .iter()
        .filter_map(|n| {
            let status = n.get("status").and_then(serde_json::Value::as_str);
            if status == Some("active") || status == Some("online") {
                n.get("hostname")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| n.get("id").and_then(serde_json::Value::as_str))
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(result)
}

fn merge_workloads(
    candidates: Vec<DurableAgentWorkloadRecord>,
) -> Option<DurableAgentWorkloadRecord> {
    candidates.into_iter().max_by(|a, b| {
        a.committed_at_ms
            .cmp(&b.committed_at_ms)
            .then_with(|| a.record_digest.cmp(&b.record_digest))
    })
}

fn parse_workload_status(stdout: &str) -> Option<DurableAgentWorkloadRecord> {
    #[derive(serde::Deserialize)]
    struct StatusOutput {
        durable_record: Option<DurableAgentWorkloadRecord>,
    }
    serde_json::from_str::<StatusOutput>(stdout)
        .ok()
        .and_then(|s| s.durable_record)
}

fn parse_workload_list(stdout: &str) -> Result<Vec<DurableAgentWorkloadRecord>, String> {
    #[derive(serde::Deserialize)]
    struct ListOutput {
        workloads: Vec<DurableAgentWorkloadRecord>,
    }
    serde_json::from_str::<ListOutput>(stdout)
        .map(|l| l.workloads)
        .map_err(|e| format!("failed to parse remote list JSON: {e}"))
}

fn emit_single_record(record: &DurableAgentWorkloadRecord, json: bool) {
    if json {
        let v = serde_json::to_value(record).unwrap_or_default();
        emit(v, true);
    } else {
        println!("Workload:     {}", record.record.envelope.workload_id);
        println!("State:        {:?}", record.record.state);
        println!("Revision:     {}", record.revision);
        println!("Committed at: {} ms", record.committed_at_ms);
        println!("Digest:       {}", record.record_digest);
        println!(
            "Objective:    {}",
            record.record.envelope.delegation.objective
        );
        if let Some(claim) = &record.record.claim {
            println!("Claim Agent:  {}", claim.guard.agent_id);
            println!("Claim Scope:  {}", claim.guard.lease_id);
            println!("Expires at:   {} ms", claim.expires_at_ms);
        }
        if let Some(comp) = &record.record.completion {
            println!("Completion:   {:?}", comp.status);
            println!("Delegation:   {}", comp.delegation_id);
        }
    }
}

fn emit_record_list(records: &[DurableAgentWorkloadRecord], json: bool) {
    if json {
        let v = serde_json::to_value(records).unwrap_or_default();
        emit(v, true);
    } else {
        println!(
            "{:<16} {:<12} {:<6} {:<16} {:<32}",
            "WORKLOAD ID", "STATE", "REV", "COMMITTED AT", "OBJECTIVE"
        );
        println!("{}", "-".repeat(84));
        for r in records {
            let obj = if r.record.envelope.delegation.objective.len() > 30 {
                format!("{}...", &r.record.envelope.delegation.objective[..27])
            } else {
                r.record.envelope.delegation.objective.clone()
            };
            println!(
                "{:<16} {:<12?} {:<6} {:<16} {:<32}",
                r.record.envelope.workload_id, r.record.state, r.revision, r.committed_at_ms, obj
            );
        }
    }
}

fn emit(value: serde_json::Value, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&value).unwrap_or_default()
        );
    } else {
        println!("{value}");
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        AGENT_DELEGATION_ENVELOPE_SCHEMA, AGENT_WORKLOAD_ENVELOPE_SCHEMA,
        AGENT_WORKLOAD_RECORD_SCHEMA, AgentBoundedAction, AgentBudget, AgentCapabilityGrant,
        AgentDelegationEnvelope, AgentEvidenceRef, AgentWorkloadEnvelope, AgentWorkloadRecord,
        AgentWorkloadState, EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization,
        MutationContract, POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority,
        PolicyDecision, PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof,
    };

    fn dummy_envelope() -> AgentWorkloadEnvelope {
        let action = AgentBoundedAction {
            argv: vec!["make".to_string(), "check".to_string()],
            working_directory: Some("checkout".to_string()),
            input_digest: Some(format!("sha256:{}", "b".repeat(64))),
            mutates: true,
            requires_privilege: false,
            stop_condition: "stop on first non-zero exit".to_string(),
            rollback_intent: "discard isolated build checkout".to_string(),
        };
        let mutation = MutationContract {
            kind: "repository_validation".to_string(),
            scope: PolicyScope {
                domain: "build".to_string(),
                target: "commit-a".to_string(),
            },
            mutates: true,
            requires_privilege: false,
            requires_explicit_apply: true,
            required_authority: PolicyAuthority::RuntimePrincipal,
            preconditions: vec!["source_commit_observed".to_string()],
            rollback_intent: "discard isolated build checkout".to_string(),
            expected_proofs: vec![RequiredProof {
                gate: "canonical_gate_report".to_string(),
                subject: "commit-a".to_string(),
            }],
        };
        AgentWorkloadEnvelope {
            schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.to_string(),
            workload_id: "w1".to_string(),
            idempotency_key: "k1".to_string(),
            plan_digest: format!("sha256:{}", "a".repeat(64)),
            mandate_binding: None,
            input: None,
            action,
            delegation: AgentDelegationEnvelope {
                schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.to_string(),
                delegation_id: "d1".to_string(),
                node_id: "n1".to_string(),
                agent_id: "a1".to_string(),
                parent_agent_id: None,
                depth: 0,
                objective: "build".to_string(),
                authority_snapshot: AgentEvidenceRef {
                    kind: "test".to_string(),
                    subject: "s1".to_string(),
                    digest: "dig1".to_string(),
                    lineage: None,
                },
                context_refs: vec![],
                capabilities: vec![AgentCapabilityGrant {
                    id: "repository_validation".to_string(),
                    mutates: true,
                }],
                budget: AgentBudget {
                    context_tokens: 0,
                    actions: 0,
                    children: 0,
                    wall_time_ms: 0,
                    cpu_ms: None,
                },
                expected_proofs: vec!["canonical_gate_report".to_string()],
            },
            parent_delegation: None,
            policy_admission: PolicyAdmissionReport {
                schema: POLICY_ADMISSION_REPORT_SCHEMA.to_string(),
                policy_set: PolicySetRef {
                    id: "build.validation".to_string(),
                    version: "v1".to_string(),
                    digest: format!("sha256:{}", "c".repeat(64)),
                },
                intent: PolicyIntentRef {
                    id: "validate-commit-a".to_string(),
                    kind: "repository_validation".to_string(),
                    actor: "build-agent-1".to_string(),
                    subject: "commit-a".to_string(),
                    digest: format!("sha256:{}", "d".repeat(64)),
                    plan_digest: format!("sha256:{}", "a".repeat(64)),
                },
                mutation: mutation.clone(),
                decision: PolicyDecision::Allow {
                    required_authority: PolicyAuthority::RuntimePrincipal,
                    required_proofs: mutation.expected_proofs.clone(),
                },
            },
            execution_authorization: ExecutionAuthorization {
                schema: EXECUTION_AUTHORIZATION_SCHEMA.to_string(),
                actor: "build-agent-1".to_string(),
                authority: PolicyAuthority::RuntimePrincipal,
                explicit_apply: true,
                satisfied_preconditions: vec!["source_commit_observed".to_string()],
            },
        }
    }

    fn dummy_record() -> DurableAgentWorkloadRecord {
        DurableAgentWorkloadRecord {
            schema: "test".to_string(),
            revision: 1,
            previous_record_digest: None,
            record_digest: "dummy".to_string(),
            record: AgentWorkloadRecord {
                schema: AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
                envelope: dummy_envelope(),
                state: AgentWorkloadState::Dispatched,
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
    fn merge_lww_picks_most_recent() {
        let older = dummy_record();
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
