use crate::{
    AGENT_DELEGATION_ENVELOPE_SCHEMA, AGENT_WORKLOAD_ENVELOPE_SCHEMA, AGENT_WORKLOAD_INPUT_SCHEMA,
    AgentBoundedAction, AgentBudget, AgentCapabilityGrant, AgentDelegationEnvelope,
    AgentEvidenceLineage, AgentEvidenceRef, AgentWorkloadEnvelope, AgentWorkloadInput,
    EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization, MutationContract,
    POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority, PolicyDecision,
    PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof, digest_json,
};

use super::{
    BUILD_AGENT_REQUEST_SCHEMA, BUILD_SOURCE_REFERENCE_SCHEMA,
    BUILD_SOURCE_SNAPSHOT_REFERENCE_SCHEMA, BuildAgentRequest, BuildSourceReference,
    BuildSourceSnapshotReference, admit_build_agent_request, canonical_argv_from_build_action,
    validate_build_source_reference,
};

pub fn build_agent_request_from_workload(
    envelope: &AgentWorkloadEnvelope,
) -> Result<BuildAgentRequest, Vec<String>> {
    let Some(input) = &envelope.input else {
        return Err(vec!["build_workload_input_missing".to_string()]);
    };
    if input.schema != AGENT_WORKLOAD_INPUT_SCHEMA || input.kind != BUILD_AGENT_REQUEST_SCHEMA {
        return Err(vec!["build_workload_input_schema_mismatch".to_string()]);
    }
    let digest = digest_json(&input.payload).ok();
    if digest.as_deref() != Some(input.digest.as_str())
        || input.digest != envelope.delegation.authority_snapshot.digest
        || input.digest != envelope.policy_admission.intent.digest
    {
        return Err(vec!["build_workload_request_digest_mismatch".to_string()]);
    }
    let request = serde_json::from_value::<BuildAgentRequest>(input.payload.clone())
        .map_err(|_| vec!["build_workload_request_invalid".to_string()])?;
    let admission = admit_build_agent_request(&request);
    if !admission.blockers.is_empty() {
        return Err(admission.blockers);
    }
    let source = build_source_reference_from_workload(envelope)?;
    if request.source != source
        || request.requested_by != envelope.delegation.agent_id
        || request.source_commit != envelope.policy_admission.intent.subject
    {
        return Err(vec!["build_workload_request_binding_mismatch".to_string()]);
    }
    Ok(request)
}

pub fn build_source_reference_from_workload(
    envelope: &AgentWorkloadEnvelope,
) -> Result<BuildSourceReference, Vec<String>> {
    let repositories = envelope
        .delegation
        .context_refs
        .iter()
        .filter(|reference| reference.kind == "source_repository")
        .collect::<Vec<_>>();
    let commits = envelope
        .delegation
        .context_refs
        .iter()
        .filter(|reference| reference.kind == "source_commit")
        .collect::<Vec<_>>();
    let snapshots = envelope
        .delegation
        .context_refs
        .iter()
        .filter(|reference| reference.kind == "source_snapshot")
        .collect::<Vec<_>>();
    let snapshot_sources = envelope
        .delegation
        .context_refs
        .iter()
        .filter(|reference| reference.kind == "source_snapshot_transport")
        .collect::<Vec<_>>();
    let ([repository], [commit]) = (repositories.as_slice(), commits.as_slice()) else {
        return Err(vec!["build_workload_source_context_ambiguous".to_string()]);
    };
    if snapshots.len() > 1 || snapshot_sources.len() > 1 || snapshot_sources.len() > snapshots.len()
    {
        return Err(vec![
            "build_workload_snapshot_context_ambiguous".to_string(),
        ]);
    }
    let source = BuildSourceReference {
        schema: BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
        repository_id: commit
            .lineage
            .as_ref()
            .map(|lineage| lineage.producer_id.clone())
            .unwrap_or_default(),
        transport_uri: repository.subject.clone(),
        revision: commit.subject.clone(),
        authority_ref: repository
            .lineage
            .as_ref()
            .map(|lineage| lineage.producer_id.clone())
            .unwrap_or_default(),
        snapshot: snapshots.first().map(|snapshot| {
            let source = snapshot_sources
                .first()
                .and_then(|snapshot_source| serde_json::from_str(&snapshot_source.subject).ok());
            BuildSourceSnapshotReference {
                // v1 (schema without transport evidence) was retired: a
                // snapshot whose recovery source cannot be decoded now stays
                // on the current schema and fails admission downstream with
                // build_source_snapshot_recovery_source_invalid.
                schema: BUILD_SOURCE_SNAPSHOT_REFERENCE_SCHEMA.to_string(),
                digest: snapshot.subject.clone(),
                format: crate::SOURCE_MATERIALIZATION_FORMAT.to_string(),
                authority_ref: snapshot
                    .lineage
                    .as_ref()
                    .map(|lineage| lineage.producer_id.clone())
                    .unwrap_or_default(),
                source,
            }
        }),
    };
    let mut blockers = validate_build_source_reference(&source);
    if source.revision != envelope.policy_admission.intent.subject {
        blockers.push("build_workload_source_intent_mismatch".to_string());
    }
    let digest = digest_json(&source).ok();
    if digest.as_deref() != Some(repository.digest.as_str())
        || digest.as_deref() != Some(commit.digest.as_str())
        || snapshots
            .iter()
            .any(|snapshot| digest.as_deref() != Some(snapshot.digest.as_str()))
        || snapshot_sources
            .iter()
            .any(|source| digest.as_deref() != Some(source.digest.as_str()))
        || digest.as_deref() != envelope.action.input_digest.as_deref()
    {
        blockers.push("build_workload_source_digest_mismatch".to_string());
    }
    if blockers.is_empty() {
        Ok(source)
    } else {
        blockers.sort();
        blockers.dedup();
        Err(blockers)
    }
}

pub fn compose_build_agent_workload(
    request: &BuildAgentRequest,
    target_node_id: &str,
    workspace_root: &str,
) -> Result<AgentWorkloadEnvelope, Vec<String>> {
    let admission = admit_build_agent_request(request);
    let mut blockers = admission.blockers;
    if target_node_id.trim().is_empty() {
        blockers.push("build_agent_workload_target_node_missing".to_string());
    }
    if workspace_root.trim().is_empty() {
        blockers.push("build_agent_workload_directory_missing".to_string());
    }
    if !blockers.is_empty() {
        return Err(blockers);
    }
    let argv = canonical_argv_from_build_action(&request.action)
        .ok_or_else(|| vec!["build_agent_request_action_not_allowed".to_string()])?;
    let input_digest = digest_json(&request.source)
        .map_err(|_| vec!["build_agent_workload_input_digest_failed".to_string()])?;
    let request_digest = digest_json(request)
        .map_err(|_| vec!["build_agent_workload_request_digest_failed".to_string()])?;
    let suffix = request_digest.trim_start_matches("sha256:");
    let short = suffix.chars().take(24).collect::<String>();
    let workload_id = format!("build-{short}");
    let action = AgentBoundedAction {
        argv,
        working_directory: Some(format!(
            "{}/{}/{}",
            workspace_root.trim_end_matches('/'),
            request.source.repository_id,
            workload_id
        )),
        input_digest: Some(input_digest.clone()),
        mutates: true,
        requires_privilege: false,
        stop_condition: "stop on first non-zero gate exit and retain read-back evidence"
            .to_string(),
        rollback_intent: "discard the isolated source workspace and build artifacts".to_string(),
    };
    let plan_digest = digest_json(&action)
        .map_err(|_| vec!["build_agent_workload_plan_digest_failed".to_string()])?;
    let policy_digest = digest_json(&serde_json::json!({
        "id": "build.repository_validation",
        "version": "v2",
        "actions": ["make:check", "make:build", "make:build-release"],
        "authority": "runtime_principal",
    }))
    .map_err(|_| vec!["build_agent_workload_policy_digest_failed".to_string()])?;
    let proofs = request
        .proof_requirements
        .iter()
        .map(|gate| RequiredProof {
            gate: gate.clone(),
            subject: request.source_commit.clone(),
        })
        .collect::<Vec<_>>();
    let mutation = MutationContract {
        kind: "repository_validation".to_string(),
        scope: PolicyScope {
            domain: "build".to_string(),
            target: request.source_commit.clone(),
        },
        mutates: true,
        requires_privilege: false,
        requires_explicit_apply: true,
        required_authority: PolicyAuthority::RuntimePrincipal,
        preconditions: vec!["source_commit_observed".to_string()],
        rollback_intent: action.rollback_intent.clone(),
        expected_proofs: proofs.clone(),
    };
    Ok(AgentWorkloadEnvelope {
        schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.to_string(),
        workload_id: workload_id.clone(),
        idempotency_key: format!("build-request-{short}"),
        plan_digest: plan_digest.clone(),
        input: Some(AgentWorkloadInput {
            schema: AGENT_WORKLOAD_INPUT_SCHEMA.to_string(),
            kind: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            digest: request_digest.clone(),
            payload: serde_json::to_value(request)
                .map_err(|_| vec!["build_agent_workload_request_encode_failed".to_string()])?,
        }),
        action,
        delegation: AgentDelegationEnvelope {
            schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.to_string(),
            delegation_id: format!("delegation-{short}"),
            node_id: target_node_id.to_string(),
            agent_id: request.requested_by.clone(),
            parent_agent_id: None,
            depth: 0,
            objective: format!(
                "execute {} for source commit {}",
                request.action, request.source_commit
            ),
            authority_snapshot: AgentEvidenceRef {
                kind: "build_agent_request".to_string(),
                subject: request.request_id.clone(),
                digest: request_digest.clone(),
                lineage: None,
            },
            context_refs: build_source_context_refs(request, &input_digest),
            capabilities: vec![AgentCapabilityGrant {
                id: "repository_validation".to_string(),
                mutates: true,
            }],
            budget: AgentBudget {
                context_tokens: 0,
                actions: 1,
                children: 0,
                wall_time_ms: 900_000,
                cpu_ms: None,
            },
            expected_proofs: request.proof_requirements.clone(),
        },
        parent_delegation: None,
        policy_admission: PolicyAdmissionReport {
            schema: POLICY_ADMISSION_REPORT_SCHEMA.to_string(),
            policy_set: PolicySetRef {
                id: "build.repository_validation".to_string(),
                version: "v2".to_string(),
                digest: policy_digest,
            },
            intent: PolicyIntentRef {
                id: request.request_id.clone(),
                kind: "repository_validation".to_string(),
                actor: request.requested_by.clone(),
                subject: request.source_commit.clone(),
                digest: request_digest,
                plan_digest,
            },
            mutation: mutation.clone(),
            decision: PolicyDecision::Allow {
                required_authority: PolicyAuthority::RuntimePrincipal,
                required_proofs: proofs,
            },
        },
        execution_authorization: ExecutionAuthorization {
            schema: EXECUTION_AUTHORIZATION_SCHEMA.to_string(),
            actor: request.requested_by.clone(),
            authority: PolicyAuthority::RuntimePrincipal,
            explicit_apply: request.mutation_authorized,
            satisfied_preconditions: vec!["source_commit_observed".to_string()],
        },
    })
}

fn build_source_context_refs(
    request: &BuildAgentRequest,
    input_digest: &str,
) -> Vec<AgentEvidenceRef> {
    let lineage = |producer_id: String| AgentEvidenceLineage {
        producer_id,
        producer_attestation_digest: input_digest.to_string(),
        origin_digests: vec![input_digest.to_string()],
        derivation_digests: Vec::new(),
        influence_digests: Vec::new(),
    };
    let mut refs = vec![
        AgentEvidenceRef {
            kind: "source_repository".to_string(),
            subject: request.source.transport_uri.clone(),
            digest: input_digest.to_string(),
            lineage: Some(lineage(request.source.authority_ref.clone())),
        },
        AgentEvidenceRef {
            kind: "source_commit".to_string(),
            subject: request.source_commit.clone(),
            digest: input_digest.to_string(),
            lineage: Some(lineage(request.source.repository_id.clone())),
        },
    ];
    if let Some(snapshot) = &request.source.snapshot {
        refs.push(AgentEvidenceRef {
            kind: "source_snapshot".to_string(),
            subject: snapshot.digest.clone(),
            digest: input_digest.to_string(),
            lineage: Some(lineage(snapshot.authority_ref.clone())),
        });
        if let Some(source) = &snapshot.source {
            refs.push(AgentEvidenceRef {
                kind: "source_snapshot_transport".to_string(),
                subject: serde_json::to_string(source)
                    .expect("admitted snapshot source is serializable"),
                digest: input_digest.to_string(),
                lineage: Some(lineage(snapshot.authority_ref.clone())),
            });
        }
    }
    refs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BUILD_AGENT_REQUEST_SCHEMA;

    #[test]
    fn dispatch_composes_one_admitted_content_addressed_workload() {
        let request = BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "validate-abc".to_string(),
            requested_by: "codex-session-1".to_string(),
            source_commit: "a".repeat(40),
            source: crate::BuildSourceReference {
                schema: crate::BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: "cynic".to_string(),
                transport_uri: "ssh://forgejo@forgejo.synapse.cynic:22/owner/CYNIC.git".into(),
                revision: "a".repeat(40),
                authority_ref: "infra/repo-remotes.json:forgejo".into(),
                snapshot: None,
            },
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: true,
            proof_requirements: vec!["canonical_gate_report".to_string()],
            release_publication: None,
            evidence_refs: vec!["forgejo:refs/heads/change@aaaaaaaa".to_string()],
        };

        let envelope = compose_build_agent_workload(
            &request,
            "cynic-builder",
            "/var/lib/cynic/source/workspaces/build-test",
        )
        .unwrap();

        assert_eq!(envelope.action.argv, vec!["make", "check"]);
        assert_eq!(envelope.delegation.node_id, "cynic-builder");
        assert!(crate::admit_agent_workload(&envelope).admitted());
        assert_eq!(
            build_source_reference_from_workload(&envelope).unwrap(),
            request.source
        );
    }

    #[test]
    fn snapshot_reference_round_trips_through_workload_context() {
        let snapshot = crate::BuildSourceSnapshotReference {
            schema: crate::BUILD_SOURCE_SNAPSHOT_REFERENCE_SCHEMA.to_string(),
            digest: format!("sha256:{}", "c".repeat(64)),
            format: crate::SOURCE_MATERIALIZATION_FORMAT.to_string(),
            authority_ref: "source-release:release-abc".to_string(),
            source: Some(crate::RuntimeArtifactSource {
                transport: crate::RuntimeArtifactTransportKind::Https,
                locator: "https://cynic-storage-primary.synapse.cynic/source-snapshots/cccc.bundle"
                    .to_string(),
            }),
        };
        let request = BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "validate-snapshot".to_string(),
            requested_by: "codex-session-1".to_string(),
            source_commit: "a".repeat(40),
            source: crate::BuildSourceReference {
                schema: crate::BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: "cynic".to_string(),
                transport_uri: "ssh://forgejo@forgejo.synapse.cynic:22/owner/CYNIC.git".into(),
                revision: "a".repeat(40),
                authority_ref: "infra/repo-remotes.json:forgejo".into(),
                snapshot: Some(snapshot.clone()),
            },
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: true,
            proof_requirements: vec!["source_snapshot_digest_read_back".to_string()],
            release_publication: None,
            evidence_refs: vec![snapshot.digest.clone()],
        };

        let envelope = compose_build_agent_workload(
            &request,
            "cynic-builder",
            "/var/lib/cynic/source/workspaces/build-test",
        )
        .expect("snapshot workload");

        assert_eq!(
            build_source_reference_from_workload(&envelope)
                .expect("source")
                .snapshot,
            Some(snapshot)
        );
        assert_eq!(
            envelope
                .delegation
                .context_refs
                .iter()
                .filter(|reference| reference.kind == "source_snapshot")
                .count(),
            1
        );
        assert_eq!(
            envelope
                .delegation
                .context_refs
                .iter()
                .filter(|reference| reference.kind == "source_snapshot_transport")
                .count(),
            1
        );
    }

    #[test]
    fn build_request_payload_tampering_is_rejected_before_execution() {
        let request = BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "validate-tamper".to_string(),
            requested_by: "codex-session-1".to_string(),
            source_commit: "a".repeat(40),
            source: crate::BuildSourceReference {
                schema: crate::BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: "cynic".to_string(),
                transport_uri: "ssh://forgejo@forgejo.synapse.cynic:22/owner/CYNIC.git".into(),
                revision: "a".repeat(40),
                authority_ref: "infra/repo-remotes.json:forgejo".into(),
                snapshot: None,
            },
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: true,
            proof_requirements: vec!["canonical_gate_report".to_string()],
            release_publication: None,
            evidence_refs: vec!["forgejo:refs/heads/change@aaaaaaaa".to_string()],
        };
        let mut envelope = compose_build_agent_workload(
            &request,
            "cynic-builder",
            "/var/lib/cynic/source/workspaces/build-test",
        )
        .unwrap();
        envelope.input.as_mut().unwrap().payload["action"] =
            serde_json::Value::String("make:build-release".to_string());

        assert!(!crate::admit_agent_workload(&envelope).admitted());
        assert!(build_agent_request_from_workload(&envelope).is_err());
    }

    #[test]
    fn release_build_is_one_bounded_scheduler_action() {
        let request = BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "release-abc".to_string(),
            requested_by: "release-controller".to_string(),
            source_commit: "b".repeat(40),
            source: crate::BuildSourceReference {
                schema: crate::BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: "cynic".to_string(),
                transport_uri: "ssh://forgejo@forgejo.synapse.cynic:22/owner/CYNIC.git".into(),
                revision: "b".repeat(40),
                authority_ref: "infra/repo-remotes.json:forgejo".into(),
                snapshot: None,
            },
            action: "make:build-release".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: true,
            proof_requirements: vec![
                "release_binary_artifact_read_back".to_string(),
                "signed_release_basket_read_back".to_string(),
            ],
            release_publication: Some(crate::BuildReleasePublicationIntent {
                schema: crate::BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA.to_string(),
                release_id: "release-abc".to_string(),
                release_epoch: 1,
            }),
            evidence_refs: vec!["forgejo:refs/heads/main@bbbbbbbb".to_string()],
        };

        let envelope = compose_build_agent_workload(
            &request,
            "cynic-builder",
            "/var/lib/cynic/source/workspaces",
        )
        .expect("release workload");

        assert_eq!(envelope.action.argv, vec!["make", "build-release"]);
        assert_eq!(envelope.delegation.budget.actions, 1);
        assert_eq!(
            build_agent_request_from_workload(&envelope).unwrap(),
            request
        );
    }
}
