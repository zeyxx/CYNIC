//! Node-initiated build pull: select runnable check-phase build workloads.
//!
//! Pure reconcile over the durable workload store plus delivery leases. It
//! never spawns, never mutates, never acquires leases: lease lifecycle stays
//! with the delivery use case. Only `cargo check` argv is eligible — later
//! phases mutate and need their own admission slice. A missing or expired
//! lease skips the workload with an explicit reason instead of running it.

use cynic_protocol::{AgentWorkloadState, DurableAgentWorkloadRecord};

use crate::ports::agent_workload_delivery_lease_port::AgentWorkloadDeliveryLeasePort;

pub const BUILD_PULL_PLAN_SCHEMA: &str = "cynic.node.build_pull_plan.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildPullItem {
    pub workload_id: String,
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildPullPlan {
    pub schema: String,
    pub node_id: String,
    pub runnable: Vec<BuildPullItem>,
    pub skipped_terminal: u64,
    pub skipped_no_lease: u64,
    pub skipped_not_build: u64,
    pub blockers: Vec<String>,
}

fn is_check_argv(argv: &[String]) -> bool {
    let mut parts = argv.iter().map(String::as_str);
    match parts.next().map(|program| {
        std::path::Path::new(program)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
    }) {
        Some("cargo") => parts.next() == Some("check"),
        _ => false,
    }
}

/// Select the check-phase build workloads this node may run now.
///
/// `now_ms` bounds lease validity. Every skip is counted by cause; lease
/// read failures block the plan loudly instead of silently dropping work.
pub fn select_build_pull(
    node_id: &str,
    now_ms: u64,
    records: &[DurableAgentWorkloadRecord],
    leases: &dyn AgentWorkloadDeliveryLeasePort,
) -> BuildPullPlan {
    let mut plan = BuildPullPlan {
        schema: BUILD_PULL_PLAN_SCHEMA.to_string(),
        node_id: node_id.to_string(),
        runnable: Vec::new(),
        skipped_terminal: 0,
        skipped_no_lease: 0,
        skipped_not_build: 0,
        blockers: Vec::new(),
    };
    for durable in records {
        let workload_id = durable.record.envelope.workload_id.clone();
        if durable.record.state.terminal() {
            plan.skipped_terminal += 1;
            continue;
        }
        if !is_check_argv(&durable.record.envelope.action.argv) {
            plan.skipped_not_build += 1;
            continue;
        }
        match leases.read(&workload_id) {
            Ok(Some(lease))
                if lease.consumer_instance_id == node_id && lease.expires_at_ms > now_ms =>
            {
                plan.runnable.push(BuildPullItem {
                    workload_id,
                    argv: durable.record.envelope.action.argv.clone(),
                });
            }
            Ok(_) => plan.skipped_no_lease += 1,
            Err(error) => plan.blockers.push(format!(
                "build_pull_lease_read_failed:{workload_id}:{error}"
            )),
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        AGENT_DELEGATION_ENVELOPE_SCHEMA, AGENT_WORKLOAD_ENVELOPE_SCHEMA,
        AGENT_WORKLOAD_RECORD_SCHEMA, AgentBoundedAction, AgentBudget, AgentCapabilityGrant,
        AgentDelegationEnvelope, AgentEvidenceRef, AgentWorkloadDeliveryLease,
        AgentWorkloadEnvelope, AgentWorkloadRecord, DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA,
        EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization, MutationContract,
        POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority, PolicyDecision,
        PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof, digest_json,
    };
    use std::collections::HashMap;

    struct MapLeases {
        entries: HashMap<String, AgentWorkloadDeliveryLease>,
        fail: Vec<String>,
    }

    impl AgentWorkloadDeliveryLeasePort for MapLeases {
        fn read(&self, workload_id: &str) -> Result<Option<AgentWorkloadDeliveryLease>, String> {
            if self.fail.iter().any(|id| id == workload_id) {
                return Err("store offline".to_string());
            }
            Ok(self.entries.get(workload_id).cloned())
        }

        fn persist(
            &self,
            _workload_id: &str,
            _lease: &AgentWorkloadDeliveryLease,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    fn lease(holder: &str, expires_at_ms: u64) -> AgentWorkloadDeliveryLease {
        AgentWorkloadDeliveryLease {
            schema: "cynic.agent_workload_delivery_lease.v1".to_string(),
            lease_id: format!("lease-{holder}"),
            fencing_token: 1,
            consumer_instance_id: holder.to_string(),
            accepted_at_ms: 1,
            expires_at_ms,
            record_digest: "sha256:abc".to_string(),
        }
    }

    fn envelope(workload_id: &str, argv: Vec<&str>) -> AgentWorkloadEnvelope {
        let action = AgentBoundedAction {
            argv: argv.into_iter().map(str::to_string).collect(),
            working_directory: Some("checkout".into()),
            input_digest: Some(format!("sha256:{}", "b".repeat(64))),
            mutates: false,
            requires_privilege: false,
            stop_condition: "stop on first non-zero exit".into(),
            rollback_intent: "discard isolated checkout".into(),
        };
        let plan_digest = digest_json(&action).expect("action digest");
        AgentWorkloadEnvelope {
            schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.into(),
            workload_id: workload_id.into(),
            idempotency_key: format!("key-{workload_id}"),
            plan_digest: plan_digest.clone(),
            input: None,
            action,
            delegation: AgentDelegationEnvelope {
                schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.into(),
                delegation_id: format!("delegation-{workload_id}"),
                node_id: "builder-1".into(),
                agent_id: "build-agent-1".into(),
                parent_agent_id: None,
                depth: 0,
                objective: "run canonical check".into(),
                authority_snapshot: AgentEvidenceRef {
                    kind: "authority_snapshot".into(),
                    subject: workload_id.into(),
                    digest: format!("sha256:{}", "e".repeat(64)),
                    lineage: None,
                },
                context_refs: vec![],
                capabilities: vec![AgentCapabilityGrant {
                    id: "repository_validation".into(),
                    mutates: false,
                }],
                budget: AgentBudget {
                    context_tokens: 1_000,
                    actions: 2,
                    children: 0,
                    wall_time_ms: 900_000,
                    cpu_ms: None,
                },
                expected_proofs: vec!["canonical_gate_report".into()],
            },
            parent_delegation: None,
            policy_admission: PolicyAdmissionReport {
                schema: POLICY_ADMISSION_REPORT_SCHEMA.into(),
                policy_set: PolicySetRef {
                    id: "build.validation".into(),
                    version: "v1".into(),
                    digest: format!("sha256:{}", "c".repeat(64)),
                },
                intent: PolicyIntentRef {
                    id: format!("key-{workload_id}"),
                    kind: "repository_validation".into(),
                    actor: "build-agent-1".into(),
                    subject: "commit-a".into(),
                    digest: format!("sha256:{}", "d".repeat(64)),
                    plan_digest,
                },
                mutation: MutationContract {
                    kind: "repository_validation".into(),
                    scope: PolicyScope {
                        domain: "build".into(),
                        target: "commit-a".into(),
                    },
                    mutates: false,
                    requires_privilege: false,
                    requires_explicit_apply: false,
                    required_authority: PolicyAuthority::RuntimePrincipal,
                    preconditions: vec!["source_commit_observed".into()],
                    rollback_intent: "discard isolated checkout".into(),
                    expected_proofs: vec![RequiredProof {
                        gate: "canonical_gate_report".into(),
                        subject: "commit-a".into(),
                    }],
                },
                decision: PolicyDecision::Allow {
                    required_authority: PolicyAuthority::RuntimePrincipal,
                    required_proofs: vec![],
                },
            },
            execution_authorization: ExecutionAuthorization {
                schema: EXECUTION_AUTHORIZATION_SCHEMA.into(),
                actor: "build-agent-1".into(),
                authority: PolicyAuthority::RuntimePrincipal,
                explicit_apply: false,
                satisfied_preconditions: vec!["source_commit_observed".into()],
            },
        }
    }

    fn record(
        workload_id: &str,
        state: AgentWorkloadState,
        argv: Vec<&str>,
    ) -> DurableAgentWorkloadRecord {
        DurableAgentWorkloadRecord {
            schema: DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.into(),
            revision: 1,
            previous_record_digest: None,
            record_digest: "sha256:abc".into(),
            record: AgentWorkloadRecord {
                schema: AGENT_WORKLOAD_RECORD_SCHEMA.into(),
                envelope: envelope(workload_id, argv),
                state,
                claim: None,
                last_fencing_token: 0,
                cancellation_generation: 0,
                consumed_actions: 0,
                completion: None,
            },
            committed_at_ms: 1,
        }
    }

    fn check_argv() -> Vec<&'static str> {
        vec!["cargo", "check", "--package", "cynic-host", "--lib"]
    }

    #[test]
    fn selects_held_check_workload() {
        let mut entries = HashMap::new();
        entries.insert("w1".to_string(), lease("node-1", 999));
        let leases = MapLeases {
            entries,
            fail: Vec::new(),
        };
        let records = vec![record("w1", AgentWorkloadState::Dispatched, check_argv())];

        let plan = select_build_pull("node-1", 100, &records, &leases);

        assert!(plan.blockers.is_empty());
        assert_eq!(plan.runnable.len(), 1);
        assert_eq!(plan.runnable[0].workload_id, "w1");
    }

    #[test]
    fn skips_terminal_other_holder_expired_and_non_build() {
        let mut entries = HashMap::new();
        entries.insert("other".to_string(), lease("node-9", 999));
        entries.insert("expired".to_string(), lease("node-1", 10));
        let leases = MapLeases {
            entries,
            fail: Vec::new(),
        };
        let records = vec![
            record("done", AgentWorkloadState::Completed, check_argv()),
            record("other", AgentWorkloadState::Dispatched, check_argv()),
            record("expired", AgentWorkloadState::Dispatched, check_argv()),
            record(
                "shell",
                AgentWorkloadState::Dispatched,
                vec!["sh", "-c", "cargo check"],
            ),
        ];

        let plan = select_build_pull("node-1", 100, &records, &leases);

        assert!(plan.runnable.is_empty());
        assert_eq!(plan.skipped_terminal, 1);
        assert_eq!(plan.skipped_no_lease, 2);
        assert_eq!(plan.skipped_not_build, 1);
    }

    #[test]
    fn lease_read_failure_blocks_loudly() {
        let leases = MapLeases {
            entries: HashMap::new(),
            fail: vec!["w1".to_string()],
        };
        let records = vec![record("w1", AgentWorkloadState::Dispatched, check_argv())];

        let plan = select_build_pull("node-1", 100, &records, &leases);

        assert!(plan.runnable.is_empty());
        assert_eq!(plan.blockers.len(), 1);
    }
}
