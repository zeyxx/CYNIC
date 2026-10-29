//! Terminal workload archive execution (dry-run + --apply).
//!
//! Forward-carry rule (TOCTOU): the scheduler may settle a record between
//! the list scan and the archive call. Records with a live claim (lease not
//! expired: `expires_at_ms > now_ms`) are skipped at scan time — the lease
//! TTL bounds the race window. The authoritative re-verification is
//! `archive_terminal` (Task 4), which re-reads the latest revision,
//! re-checks terminal state and age at mutation time. Citation: "A second
//! live build settling while the archive task moves its record dir: the
//! archive must re-verify terminal state at mutation time" (T5 review focus,
//! scheduler-truths plan).

use cynic_node::ports::agent_workload_store_port::AgentWorkloadStorePort;

/// Archive eligible terminal workloads under `state_root`.
///
/// Dry-run (`apply=false`): returns eligible workload ids + ages without
/// mutation. Apply (`apply=true`): calls `archive_terminal` per eligible
/// workload, returns moved paths + absence read-backs.
pub(crate) fn archive_terminal_workloads(
    state_root: &str,
    older_than_ms: u64,
    now_ms: u64,
    apply: bool,
) -> Result<ArchiveReport, String> {
    use cynic_node::adapters::local_agent_workload_store::LocalAgentWorkloadStore;

    let store = LocalAgentWorkloadStore::new(std::path::Path::new(state_root));
    let records = store
        .durable_records()
        .map_err(|e| format!("workload store list error: {e:?}"))?;

    let mut eligible = Vec::new();
    for record in &records {
        let workload_id = &record.record.envelope.workload_id;
        let is_terminal = record.record.state.terminal();
        if !is_terminal {
            continue;
        }
        // Forward-carry: skip records with a live claim (lease not expired).
        if let Some(claim) = &record.record.claim {
            if claim.expires_at_ms > now_ms {
                continue;
            }
        }
        let age = now_ms.saturating_sub(record.committed_at_ms);
        if age < older_than_ms {
            continue;
        }
        eligible.push((workload_id.clone(), age));
    }

    let mut archived = Vec::new();
    for (workload_id, age) in &eligible {
        if !apply {
            continue;
        }
        match store.archive_terminal(workload_id, older_than_ms, now_ms) {
            Ok(Some(path)) => {
                let read_back_absent = !std::path::Path::new(state_root).join(workload_id).exists();
                archived.push(ArchivedWorkload {
                    workload_id: workload_id.clone(),
                    age_ms: *age,
                    archive_path: path,
                    source_absent: read_back_absent,
                });
            }
            Ok(None) => {
                // Record became ineligible between scan and archive (TOCTOU).
                // This is expected: archive_terminal re-verified and refused.
            }
            Err(e) => {
                return Err(format!("archive_terminal failed for {workload_id}: {e:?}"));
            }
        }
    }

    Ok(ArchiveReport {
        eligible_count: eligible.len(),
        eligible: eligible
            .into_iter()
            .map(|(id, age)| EligibleWorkload {
                workload_id: id,
                age_ms: age,
            })
            .collect(),
        archived,
        apply,
    })
}

/// One eligible workload in dry-run mode.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct EligibleWorkload {
    pub(crate) workload_id: String,
    pub(crate) age_ms: u64,
}

/// One archived workload in apply mode.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ArchivedWorkload {
    pub(crate) workload_id: String,
    pub(crate) age_ms: u64,
    pub(crate) archive_path: String,
    pub(crate) source_absent: bool,
}

/// Full archive report.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ArchiveReport {
    pub(crate) eligible_count: usize,
    pub(crate) eligible: Vec<EligibleWorkload>,
    pub(crate) archived: Vec<ArchivedWorkload>,
    pub(crate) apply: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        AGENT_DELEGATION_ENVELOPE_SCHEMA, AgentBudget, AgentCapabilityGrant,
        AgentDelegationEnvelope, AgentEvidenceRef, EXECUTION_AUTHORIZATION_SCHEMA,
        ExecutionAuthorization, MutationContract, POLICY_ADMISSION_REPORT_SCHEMA,
        PolicyAdmissionReport, PolicyAuthority, PolicyDecision, PolicyIntentRef, PolicyScope,
        PolicySetRef, RequiredProof, digest_json,
    };
    use cynic_agent::{
        AGENT_WORKLOAD_ENVELOPE_SCHEMA, AGENT_WORKLOAD_RECORD_SCHEMA, AgentBoundedAction,
        AgentWorkloadEnvelope, AgentWorkloadRecord, AgentWorkloadState,
    };

    fn test_envelope(workload_id: &str, idempotency_key: &str) -> AgentWorkloadEnvelope {
        let action = AgentBoundedAction {
            argv: vec!["cargo".into(), "check".into()],
            working_directory: Some("checkout".into()),
            input_digest: Some(format!("sha256:{}", "b".repeat(64))),
            mutates: true,
            requires_privilege: false,
            stop_condition: "stop on first non-zero exit".into(),
            rollback_intent: "discard isolated checkout".into(),
        };
        let plan_digest = digest_json(&action).expect("action digest");
        let mutation = MutationContract {
            kind: "repository_validation".into(),
            scope: PolicyScope {
                domain: "build".into(),
                target: "commit-a".into(),
            },
            mutates: true,
            requires_privilege: false,
            requires_explicit_apply: true,
            required_authority: PolicyAuthority::RuntimePrincipal,
            preconditions: vec!["source_commit_observed".into()],
            rollback_intent: "discard isolated checkout".into(),
            expected_proofs: vec![RequiredProof {
                gate: "canonical_gate_report".into(),
                subject: "commit-a".into(),
            }],
        };
        AgentWorkloadEnvelope {
            schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.into(),
            workload_id: workload_id.into(),
            idempotency_key: idempotency_key.into(),
            plan_digest: plan_digest.clone(),
            mandate_binding: None,
            input: None,
            resource_budget: None,
            source_commit: None,
            branch: None,
            action,
            delegation: AgentDelegationEnvelope {
                schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.into(),
                delegation_id: format!("delegation-{workload_id}"),
                node_id: "builder-1".into(),
                agent_id: "build-agent-1".into(),
                parent_agent_id: None,
                depth: 0,
                objective: "run canonical validation".into(),
                authority_snapshot: AgentEvidenceRef {
                    kind: "authority_snapshot".into(),
                    subject: workload_id.into(),
                    digest: format!("sha256:{}", "a".repeat(64)),
                    lineage: None,
                },
                context_refs: vec![],
                capabilities: vec![AgentCapabilityGrant {
                    id: "repository_validation".into(),
                    mutates: true,
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
                    id: idempotency_key.into(),
                    kind: "repository_validation".into(),
                    actor: "build-agent-1".into(),
                    subject: "commit-a".into(),
                    digest: format!("sha256:{}", "d".repeat(64)),
                    plan_digest,
                },
                mutation: mutation.clone(),
                decision: PolicyDecision::Allow {
                    required_authority: PolicyAuthority::RuntimePrincipal,
                    required_proofs: mutation.expected_proofs.clone(),
                },
            },
            execution_authorization: ExecutionAuthorization {
                schema: EXECUTION_AUTHORIZATION_SCHEMA.into(),
                actor: "build-agent-1".into(),
                authority: PolicyAuthority::RuntimePrincipal,
                explicit_apply: true,
                satisfied_preconditions: vec!["source_commit_observed".into()],
            },
        }
    }

    fn test_record(workload_id: &str, idempotency_key: &str) -> AgentWorkloadRecord {
        AgentWorkloadRecord {
            schema: AGENT_WORKLOAD_RECORD_SCHEMA.into(),
            envelope: test_envelope(workload_id, idempotency_key),
            state: AgentWorkloadState::Dispatched,
            claim: None,
            last_fencing_token: 0,
            cancellation_generation: 0,
            consumed_actions: 0,
            completion: None,
        }
    }

    fn test_claim(
        workload_id: &str,
        fencing_token: u64,
        expires_at_ms: u64,
    ) -> cynic_agent::AgentWorkloadClaim {
        cynic_agent::AgentWorkloadClaim {
            guard: cynic_protocol::LeaseGuard {
                agent_id: "build-agent-1".into(),
                target: workload_id.into(),
                lease_id: "lease-1".into(),
                fencing_token,
            },
            expires_at_ms,
            cancellation_generation: 0,
            max_lease_window_ms: 300_000,
            granted_at_ms: expires_at_ms.saturating_sub(300_000),
        }
    }

    fn durable_record(
        workload_id: &str,
        idempotency_key: &str,
        state: AgentWorkloadState,
        committed_at_ms: u64,
        claim: Option<u64>,
    ) -> cynic_agent::DurableAgentWorkloadRecord {
        let mut rec = test_record(workload_id, idempotency_key);
        rec.state = state;
        rec.claim = claim.map(|exp| test_claim(workload_id, rec.last_fencing_token, exp));
        cynic_agent::DurableAgentWorkloadRecord {
            schema: cynic_agent::DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
            revision: 1,
            previous_record_digest: None,
            record_digest: format!("digest-{workload_id}"),
            record: rec,
            committed_at_ms,
        }
    }

    #[test]
    fn dry_run_lists_eligible_terminal_records_without_mutation() {
        let root = tempfile::tempdir().expect("tempdir");
        let store = cynic_node::adapters::local_agent_workload_store::LocalAgentWorkloadStore::new(
            root.path(),
        );

        let mut terminal_old = durable_record(
            "workload-old",
            "key-old",
            AgentWorkloadState::Completed,
            1_000,
            None,
        );
        store
            .compare_and_swap("workload-old", None, &terminal_old.record, 1_000)
            .expect("seed terminal old");

        store
            .compare_and_swap(
                "workload-live",
                None,
                &test_record("workload-live", "key-live"),
                1_000,
            )
            .expect("seed live");

        let mut terminal_fresh = durable_record(
            "workload-fresh",
            "key-fresh",
            AgentWorkloadState::Blocked,
            4_000,
            None,
        );
        store
            .compare_and_swap("workload-fresh", None, &terminal_fresh.record, 4_000)
            .expect("seed terminal fresh");

        let mut terminal_claimed = durable_record(
            "workload-claimed",
            "key-claimed",
            AgentWorkloadState::Claimed,
            1_000,
            Some(6_000),
        );
        store
            .compare_and_swap("workload-claimed", None, &terminal_claimed.record, 1_000)
            .expect("seed terminal claimed");

        let report = archive_terminal_workloads(root.path().to_str().unwrap(), 3_000, 5_000, false)
            .expect("dry-run archive");

        assert_eq!(report.eligible_count, 1);
        assert_eq!(report.eligible.len(), 1);
        assert_eq!(report.eligible[0].workload_id, "workload-old");
        assert_eq!(report.eligible[0].age_ms, 4_000);
        assert!(report.archived.is_empty());
        assert!(!report.apply);

        assert!(root.path().join("workload-old").exists());
        assert!(root.path().join("workload-live").exists());
        assert!(root.path().join("workload-fresh").exists());
        assert!(root.path().join("workload-claimed").exists());
    }

    #[test]
    fn apply_archives_eligible_records_with_read_back() {
        let root = tempfile::tempdir().expect("tempdir");
        let store = cynic_node::adapters::local_agent_workload_store::LocalAgentWorkloadStore::new(
            root.path(),
        );

        let mut terminal_old = durable_record(
            "workload-old",
            "key-old",
            AgentWorkloadState::Completed,
            1_000,
            None,
        );
        store
            .compare_and_swap("workload-old", None, &terminal_old.record, 1_000)
            .expect("seed terminal old");

        let report = archive_terminal_workloads(root.path().to_str().unwrap(), 3_000, 5_000, true)
            .expect("apply archive");

        assert_eq!(report.eligible_count, 1);
        assert_eq!(report.archived.len(), 1);
        assert_eq!(report.archived[0].workload_id, "workload-old");
        assert!(report.archived[0].archive_path.contains("archive"));
        assert!(report.archived[0].archive_path.contains("workload-old"));
        assert!(report.archived[0].source_absent);
        assert!(report.apply);

        assert!(!root.path().join("workload-old").exists());
    }

    #[test]
    fn apply_skips_records_that_become_ineligible_during_run() {
        let root = tempfile::tempdir().expect("tempdir");
        let store = cynic_node::adapters::local_agent_workload_store::LocalAgentWorkloadStore::new(
            root.path(),
        );

        let mut terminal_old = durable_record(
            "workload-old",
            "key-old",
            AgentWorkloadState::Completed,
            1_000,
            None,
        );
        store
            .compare_and_swap("workload-old", None, &terminal_old.record, 1_000)
            .expect("seed terminal old");

        let _ = store
            .archive_terminal("workload-old", 3_000, 5_000)
            .expect("first archive");

        let report = archive_terminal_workloads(root.path().to_str().unwrap(), 3_000, 5_000, true)
            .expect("second archive");

        assert_eq!(report.eligible_count, 0);
        assert!(report.archived.is_empty());
    }
}
