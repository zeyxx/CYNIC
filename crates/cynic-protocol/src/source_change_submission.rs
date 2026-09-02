use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    admit_agent_workload, digest_json, AgentWorkloadEnvelope, AgentWorkspaceChange,
    AgentWorkspaceRetentionRoot, AgentWorkspaceZone, RuntimeArtifactSource,
};

pub const SOURCE_CHANGE_SUBMISSION_SCHEMA: &str = "cynic.source_change_submission.v1";
pub const SOURCE_CHANGE_SUBMISSION_ADMISSION_SCHEMA: &str =
    "cynic.source_change_submission_admission.v1";
pub const SOURCE_CHANGE_SCHEDULER_RUNTIME_CAPABILITIES: &[&str] = &[
    "kernel_to_node_workload_delivery_read_back",
    "scheduler_claim_renewal",
    "scheduler_cancellation_delivery",
    "scheduler_crash_recovery",
    "node_workspace_observer_wiring_and_proof",
    "workspace_claim_store_wiring_and_conformance_proof",
    "bounded_source_change_apply_adapter",
    "post_apply_source_read_back",
];

/// A content-addressed result blob that a node must acquire and verify before
/// applying a source change. MCP may submit this reference, but never executes
/// the change or becomes the blob authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceChangeBlobReference {
    pub digest: String,
    pub source: RuntimeArtifactSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceChangeSubmission {
    pub schema: String,
    pub submission_id: String,
    pub repository_id: String,
    pub workspace_id: String,
    pub base_commit: String,
    pub changes: Vec<AgentWorkspaceChange>,
    pub blobs: Vec<SourceChangeBlobReference>,
    pub zones: Vec<AgentWorkspaceZone>,
    pub requested_claim_ttl_ms: u64,
    pub retention_ms: u64,
    #[serde(default)]
    pub protected_roots: Vec<AgentWorkspaceRetentionRoot>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceChangeSubmissionAdmission {
    pub schema: String,
    pub status: String,
    pub submission_id: String,
    pub change_set_digest: Option<String>,
    pub blockers: Vec<String>,
}

pub fn admit_source_change_submission(
    submission: &SourceChangeSubmission,
    workload: &AgentWorkloadEnvelope,
) -> SourceChangeSubmissionAdmission {
    let mut blockers = Vec::new();
    if submission.schema != SOURCE_CHANGE_SUBMISSION_SCHEMA {
        blockers.push("source_change_submission_schema_mismatch".to_string());
    }
    for (field, value) in [
        ("submission_id", submission.submission_id.as_str()),
        ("repository_id", submission.repository_id.as_str()),
        ("workspace_id", submission.workspace_id.as_str()),
        ("base_commit", submission.base_commit.as_str()),
    ] {
        if value.trim().is_empty() {
            blockers.push(format!("source_change_submission_{field}_missing"));
        }
    }
    if !admit_agent_workload(workload).admitted() {
        blockers.push("source_change_submission_workload_not_admitted".to_string());
    }
    let change_set_digest = digest_json(submission).ok();
    if change_set_digest.as_deref() != workload.action.input_digest.as_deref() {
        blockers.push("source_change_submission_input_digest_mismatch".to_string());
    }
    match workload.input.as_ref() {
        Some(input)
            if input.schema == crate::AGENT_WORKLOAD_INPUT_SCHEMA
                && input.kind == "source_change_submission"
                && change_set_digest.as_deref() == Some(input.digest.as_str())
                && serde_json::from_value::<SourceChangeSubmission>(input.payload.clone())
                    .ok()
                    .as_ref()
                    == Some(submission) => {}
        _ => blockers.push("source_change_submission_workload_input_mismatch".to_string()),
    }
    if submission.changes.is_empty() {
        blockers.push("source_change_submission_changes_missing".to_string());
    } else if submission.changes.len() != 1 {
        // The node-side apply adapter is deliberately a one-path transaction.
        // Admitting a batch here would make the generic workload look runnable
        // while its bounded action cannot prove atomic read-back for every path.
        blockers.push("source_change_submission_v1_requires_one_change".to_string());
    }

    let mut result_digests = BTreeSet::new();
    for change in &submission.changes {
        if change.path.trim().is_empty() {
            blockers.push("source_change_submission_change_path_missing".to_string());
        }
        if let Some(digest) = &change.result_digest {
            if !valid_sha256_digest(digest) {
                blockers.push(format!(
                    "source_change_submission_result_digest_invalid:{digest}"
                ));
            }
            if !result_digests.insert(digest.as_str()) {
                blockers.push(format!(
                    "source_change_submission_result_digest_duplicate:{digest}"
                ));
            }
        }
    }
    let blob_digests = submission
        .blobs
        .iter()
        .filter_map(|blob| {
            if !valid_sha256_digest(&blob.digest) {
                blockers.push(format!(
                    "source_change_submission_blob_digest_invalid:{}",
                    blob.digest
                ));
            }
            if blob.source.locator.trim().is_empty() {
                blockers.push(format!(
                    "source_change_submission_blob_source_missing:{}",
                    blob.digest
                ));
            }
            Some(blob.digest.as_str())
        })
        .collect::<BTreeSet<_>>();
    if blob_digests.len() != submission.blobs.len() {
        blockers.push("source_change_submission_blob_digest_duplicate".to_string());
    }
    if result_digests != blob_digests {
        blockers.push("source_change_submission_blob_set_mismatch".to_string());
    }

    blockers.sort();
    blockers.dedup();
    SourceChangeSubmissionAdmission {
        schema: SOURCE_CHANGE_SUBMISSION_ADMISSION_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            "admitted".to_string()
        } else {
            "blocked".to_string()
        },
        submission_id: submission.submission_id.clone(),
        change_set_digest,
        blockers,
    }
}

/// Recover the exact source-change input carried by a generic workload.
/// Scheduler implementations must call this before acquiring blobs or creating
/// a worktree; arbitrary JSON is not source-change authority.
pub fn parse_source_change_submission_input(
    workload: &AgentWorkloadEnvelope,
) -> Result<SourceChangeSubmission, Vec<String>> {
    let submission = workload
        .input
        .as_ref()
        .filter(|input| input.kind == "source_change_submission")
        .and_then(|input| {
            serde_json::from_value::<SourceChangeSubmission>(input.payload.clone()).ok()
        })
        .ok_or_else(|| vec!["source_change_submission_input_unreadable".to_string()])?;
    let admission = admit_source_change_submission(&submission, workload);
    if admission.status == "admitted" {
        Ok(submission)
    } else {
        Err(admission.blockers)
    }
}

fn valid_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        AgentBoundedAction, AgentBudget, AgentCapabilityGrant, AgentDelegationEnvelope,
        AgentEvidenceRef, AgentWorkloadEnvelope, ExecutionAuthorization, MutationContract,
        PolicyAdmissionReport, PolicyAuthority, PolicyDecision, PolicyIntentRef, PolicyScope,
        PolicySetRef, RequiredProof, RuntimeArtifactTransportKind, EXECUTION_AUTHORIZATION_SCHEMA,
        POLICY_ADMISSION_REPORT_SCHEMA,
    };

    use super::*;

    fn submission() -> SourceChangeSubmission {
        let changes = vec![AgentWorkspaceChange {
            path: "crates/cynic-host/src/source_control.rs".to_string(),
            preimage_digest: Some(format!("sha256:{}", "a".repeat(64))),
            result_digest: Some(format!("sha256:{}", "b".repeat(64))),
        }];
        SourceChangeSubmission {
            schema: SOURCE_CHANGE_SUBMISSION_SCHEMA.to_string(),
            submission_id: "submission-1".to_string(),
            repository_id: "cynic".to_string(),
            workspace_id: "workspace-1".to_string(),
            base_commit: "a".repeat(40),
            changes,
            blobs: vec![SourceChangeBlobReference {
                digest: format!("sha256:{}", "b".repeat(64)),
                source: RuntimeArtifactSource {
                    transport: RuntimeArtifactTransportKind::Https,
                    locator: "https://artifact.example/source/b".to_string(),
                },
            }],
            zones: vec![AgentWorkspaceZone {
                kind: crate::AgentWorkspaceZoneKind::Exact,
                path: "crates/cynic-host/src/source_control.rs".to_string(),
            }],
            requested_claim_ttl_ms: 1_000,
            retention_ms: 1_000,
            protected_roots: Vec::new(),
            evidence_refs: vec!["source-proof".to_string()],
        }
    }

    fn workload(submission: &SourceChangeSubmission) -> AgentWorkloadEnvelope {
        let change_set_digest = digest_json(submission).unwrap();
        let action = AgentBoundedAction {
            argv: vec!["source-change".to_string()],
            working_directory: Some("workspace".to_string()),
            input_digest: Some(change_set_digest.clone()),
            mutates: true,
            requires_privilege: false,
            stop_condition: "stop on first non-zero exit".to_string(),
            rollback_intent: "discard isolated workspace".to_string(),
        };
        let plan_digest = digest_json(&action).unwrap();
        let mutation = MutationContract {
            kind: "source_change".to_string(),
            scope: PolicyScope {
                domain: "source".to_string(),
                target: "cynic".to_string(),
            },
            mutates: true,
            requires_privilege: false,
            requires_explicit_apply: true,
            required_authority: PolicyAuthority::RuntimePrincipal,
            preconditions: vec!["workspace_claim_observed".to_string()],
            rollback_intent: "discard isolated workspace".to_string(),
            expected_proofs: vec![RequiredProof {
                gate: "source_change_read_back".to_string(),
                subject: "cynic".to_string(),
            }],
        };
        let evidence = AgentEvidenceRef {
            kind: "source_snapshot".to_string(),
            subject: "cynic".to_string(),
            digest: format!("sha256:{}", "c".repeat(64)),
            lineage: None,
        };
        let workload = AgentWorkloadEnvelope {
            schema: crate::AGENT_WORKLOAD_ENVELOPE_SCHEMA.to_string(),
            workload_id: "source-transaction-1".to_string(),
            idempotency_key: "source-transaction-1".to_string(),
            plan_digest: plan_digest.clone(),
            input: Some(crate::AgentWorkloadInput {
                schema: crate::AGENT_WORKLOAD_INPUT_SCHEMA.to_string(),
                kind: "source_change_submission".to_string(),
                digest: change_set_digest.clone(),
                payload: serde_json::to_value(submission).unwrap(),
            }),
            action,
            delegation: AgentDelegationEnvelope {
                schema: crate::AGENT_DELEGATION_ENVELOPE_SCHEMA.to_string(),
                delegation_id: "delegation-1".to_string(),
                node_id: "node-1".to_string(),
                agent_id: "agent-1".to_string(),
                parent_agent_id: None,
                depth: 0,
                objective: "apply a source transaction".to_string(),
                authority_snapshot: evidence.clone(),
                context_refs: vec![evidence],
                capabilities: vec![AgentCapabilityGrant {
                    id: "source_change".to_string(),
                    mutates: true,
                }],
                budget: AgentBudget {
                    context_tokens: 100,
                    actions: 1,
                    children: 0,
                    wall_time_ms: 1_000,
                    cpu_ms: None,
                },
                expected_proofs: vec!["source_change_read_back".to_string()],
            },
            parent_delegation: None,
            policy_admission: PolicyAdmissionReport {
                schema: POLICY_ADMISSION_REPORT_SCHEMA.to_string(),
                policy_set: PolicySetRef {
                    id: "source.change".to_string(),
                    version: "v1".to_string(),
                    digest: format!("sha256:{}", "d".repeat(64)),
                },
                intent: PolicyIntentRef {
                    id: "source-transaction-1".to_string(),
                    kind: "source_change".to_string(),
                    actor: "agent-1".to_string(),
                    subject: "cynic".to_string(),
                    digest: format!("sha256:{}", "e".repeat(64)),
                    plan_digest: plan_digest.clone(),
                },
                mutation: mutation.clone(),
                decision: PolicyDecision::Allow {
                    required_authority: PolicyAuthority::RuntimePrincipal,
                    required_proofs: mutation.expected_proofs.clone(),
                },
            },
            execution_authorization: ExecutionAuthorization {
                schema: EXECUTION_AUTHORIZATION_SCHEMA.to_string(),
                actor: "agent-1".to_string(),
                authority: PolicyAuthority::RuntimePrincipal,
                explicit_apply: true,
                satisfied_preconditions: vec!["workspace_claim_observed".to_string()],
            },
        };
        workload
    }

    #[test]
    fn admits_an_exact_content_addressed_submission() {
        let submission = submission();
        assert_eq!(
            admit_source_change_submission(&submission, &workload(&submission)).status,
            "admitted"
        );
    }

    #[test]
    fn rejects_blob_references_that_do_not_cover_the_change_set() {
        let mut candidate = submission();
        candidate.blobs.clear();
        let admission = admit_source_change_submission(&candidate, &workload(&candidate));
        assert!(admission
            .blockers
            .contains(&"source_change_submission_blob_set_mismatch".to_string()));
    }

    #[test]
    fn rejects_a_batch_until_the_node_can_prove_multi_path_atomicity() {
        let mut candidate = submission();
        candidate.changes.push(AgentWorkspaceChange {
            path: "crates/cynic-host/src/runtime_seed.rs".to_string(),
            preimage_digest: Some(format!("sha256:{}", "f".repeat(64))),
            result_digest: None,
        });
        let admission = admit_source_change_submission(&candidate, &workload(&candidate));
        assert!(admission
            .blockers
            .contains(&"source_change_submission_v1_requires_one_change".to_string()));
    }

    #[test]
    fn workload_input_round_trips_only_after_submission_admission() {
        let submission = submission();
        assert_eq!(
            parse_source_change_submission_input(&workload(&submission)).unwrap(),
            submission
        );
    }
}
