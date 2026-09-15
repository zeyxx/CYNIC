//! Build x workload integration: grant derivation and delivery validation across
//! the cynic-build / cynic-protocol boundary. Moved from protocol unit tests
//! because a protocol dev-dependency on cynic-build links two protocol
//! instances (E0308). Integration home is the correct owner.

use cynic_build::build::{
    BUILD_AGENT_REQUEST_SCHEMA, BUILD_SOURCE_REFERENCE_SCHEMA,
    BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE, BuildAgentRequest, BuildSourceReference,
    SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA, SignedBuildWorkloadGrant,
    build_workload_grant_signing_bytes, compose_build_agent_workload,
    derive_build_workload_grant, validate_build_workload_grant,
    validate_signed_build_workload_grant,
};
use cynic_protocol::{
    AGENT_COMPLETION_ENVELOPE_SCHEMA, AGENT_DELEGATION_ENVELOPE_SCHEMA,
    AGENT_WORKLOAD_COMPLETION_PUBLICATION_SCHEMA, AGENT_WORKLOAD_DELIVERY_SCHEMA,
    AGENT_WORKLOAD_ENVELOPE_SCHEMA, AGENT_WORKLOAD_RECORD_SCHEMA, AgentBoundedAction,
    AgentBudget, AgentCapabilityGrant, AgentCompletionEnvelope, AgentCompletionStatus,
    AgentDelegationEnvelope, AgentEvidenceRef, AgentWorkloadCompletionPublication,
    AgentWorkloadDelivery, AgentWorkloadDeliveryAcceptance, AgentWorkloadDeliveryStatus,
    AgentWorkloadEnvelope, AgentWorkloadRecord, AgentWorkloadState,
    DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA, DurableAgentWorkloadRecord,
    EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization, KernelDeliveryEnvelope,
    LeaseGuard, MutationContract, POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport,
    PolicyAuthority, PolicyDecision, PolicyIntentRef, PolicyScope, PolicySetRef,
    RequiredProof, accept_agent_workload_delivery, agent_workload_delivery_available,
    build_evidence_envelope, claim_agent_workload, complete_agent_workload_delivery,
    consume_agent_workload_action, digest_json, parse_agent_workload_observation_read_back,
    request_agent_workload_cancellation, validate_agent_workload_completion_publication,
    validate_agent_workload_delivery, validate_agent_workload_submission_read_back,
};

fn evidence(kind: &str) -> AgentEvidenceRef {
    AgentEvidenceRef {
        kind: kind.to_string(),
        subject: "workload-1".to_string(),
        digest: format!("sha256:{}", "a".repeat(64)),
        lineage: None,
    }
}

fn test_envelope() -> AgentWorkloadEnvelope {
    let action = AgentBoundedAction {
        argv: vec!["make".to_string(), "check".to_string()],
        working_directory: Some("checkout".to_string()),
        input_digest: Some(format!("sha256:{}", "b".repeat(64))),
        mutates: true,
        requires_privilege: false,
        stop_condition: "stop on first non-zero exit".to_string(),
        rollback_intent: "discard isolated build checkout".to_string(),
    };
    let plan_digest = match digest_json(&action) {
        Ok(digest) => digest,
        Err(error) => panic!("bounded action must serialize in test: {error}"),
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
        workload_id: "workload-1".to_string(),
        idempotency_key: "validate-commit-a".to_string(),
        plan_digest: plan_digest.clone(),
        mandate_binding: None,
        input: None,
        action,
        delegation: AgentDelegationEnvelope {
            schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.to_string(),
            delegation_id: "delegation-1".to_string(),
            node_id: "builder-1".to_string(),
            agent_id: "build-agent-1".to_string(),
            parent_agent_id: None,
            depth: 0,
            objective: "run canonical validation".to_string(),
            authority_snapshot: evidence("authority_snapshot"),
            context_refs: vec![evidence("source_commit")],
            capabilities: vec![AgentCapabilityGrant {
                id: "repository_validation".to_string(),
                mutates: true,
            }],
            budget: AgentBudget {
                context_tokens: 1_000,
                actions: 2,
                children: 0,
                wall_time_ms: 900_000,
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
                plan_digest,
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

fn record() -> AgentWorkloadRecord {
    AgentWorkloadRecord {
        schema: AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        envelope: test_envelope(),
        state: AgentWorkloadState::Dispatched,
        claim: None,
        last_fencing_token: 0,
        cancellation_generation: 0,
        consumed_actions: 0,
        completion: None,
    }
}

fn guard(token: u64) -> LeaseGuard {
    LeaseGuard {
        agent_id: "build-agent-1".to_string(),
        target: "workload-1".to_string(),
        lease_id: format!("lease-{token}"),
        fencing_token: token,
    }
}

#[test]
fn build_grant_is_derived_from_the_live_durable_workload() {
    let now_unix_s = 10;
    let source_commit = "a".repeat(40);
    let mut dispatched = record();
    dispatched.envelope.policy_admission.intent.subject = source_commit.clone();
    dispatched.envelope.policy_admission.mutation.scope.target = source_commit.clone();
    let claimed = claim_agent_workload(&dispatched, guard(1), 40_000, now_unix_s * 1_000, 40_000)
        .expect("claim workload");
    let claimed = consume_agent_workload_action(&claimed, &guard(1), now_unix_s * 1_000)
        .expect("reserve build action");
    let durable = DurableAgentWorkloadRecord {
        schema: DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        revision: 2,
        previous_record_digest: Some(format!("sha256:{}", "e".repeat(64))),
        record_digest: digest_json(&claimed).expect("record digest"),
        record: claimed,
        committed_at_ms: now_unix_s * 1_000,
    };

    let grant = derive_build_workload_grant(&durable, "scheduler-1", now_unix_s)
        .expect("derive workload grant");

    assert_eq!(grant.source_commit, source_commit);
    assert_eq!(
        grant.allowed_actions,
        vec!["make:check", "cargo:check", "cargo:run", "cargo:test"]
    );
    assert_eq!(grant.workload_record_digest, durable.record_digest);
    assert!(
        validate_build_workload_grant(
            &grant,
            &durable,
            "builder-1",
            &source_commit,
            "make:check",
            now_unix_s + 1,
        )
        .is_ok()
    );

    let mut signed = SignedBuildWorkloadGrant {
        schema: SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA.to_string(),
        authority_key_id: format!("sha256:{}", "c".repeat(64)),
        signature_namespace: BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE.to_string(),
        grant,
        signature_hex: "d".repeat(128),
    };
    let signing_bytes = build_workload_grant_signing_bytes(&signed).expect("signing bytes");
    signed.signature_hex = "e".repeat(128);
    assert_eq!(
        signing_bytes,
        build_workload_grant_signing_bytes(&signed).expect("stable signing bytes")
    );
    assert!(validate_signed_build_workload_grant(&signed).is_empty());
}

#[test]
fn cancellation_or_fence_change_revokes_a_derived_build_grant() {
    let now_unix_s = 10;
    let source_commit = "b".repeat(40);
    let mut dispatched = record();
    dispatched.envelope.policy_admission.intent.subject = source_commit.clone();
    dispatched.envelope.policy_admission.mutation.scope.target = source_commit.clone();
    let claimed = claim_agent_workload(&dispatched, guard(1), 40_000, now_unix_s * 1_000, 40_000)
        .expect("claim workload");
    let claimed = consume_agent_workload_action(&claimed, &guard(1), now_unix_s * 1_000)
        .expect("reserve build action");
    let mut durable = DurableAgentWorkloadRecord {
        schema: DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        revision: 2,
        previous_record_digest: Some(format!("sha256:{}", "e".repeat(64))),
        record_digest: digest_json(&claimed).expect("record digest"),
        record: claimed,
        committed_at_ms: now_unix_s * 1_000,
    };
    let grant = derive_build_workload_grant(&durable, "scheduler-1", now_unix_s)
        .expect("derive workload grant");

    durable.record = request_agent_workload_cancellation(&durable.record).expect("cancel");
    durable.revision += 1;
    durable.previous_record_digest = Some(durable.record_digest.clone());
    durable.record_digest = digest_json(&durable.record).expect("cancelled digest");

    assert!(
        validate_build_workload_grant(
            &grant,
            &durable,
            "builder-1",
            &source_commit,
            "make:check",
            now_unix_s + 1,
        )
        .is_err()
    );
}

fn delivery() -> AgentWorkloadDelivery {
    let request = BuildAgentRequest {
        schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
        request_id: "delivery-test".to_string(),
        requested_by: "agent-test".to_string(),
        source_commit: "a".repeat(40),
        source: BuildSourceReference {
            schema: BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
            repository_id: "cynic".into(),
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
        evidence_refs: vec!["git:commit:test".to_string()],
    };
    AgentWorkloadDelivery {
        schema: AGENT_WORKLOAD_DELIVERY_SCHEMA.to_string(),
        envelope: compose_build_agent_workload(
            &request,
            "cynic-builder",
            "/var/lib/cynic/source/workspaces",
        )
        .unwrap(),
        target_node_id: "cynic-builder".to_string(),
        submitted_by: "agent-test".to_string(),
        submitted_at_ms: 1,
        status: AgentWorkloadDeliveryStatus::Pending,
        accepted_at_ms: None,
        accepted_record_digest: None,
        acceptance_lease: None,
        terminal_at_ms: None,
        terminal_record_digest: None,
        terminal_record: None,
        completion: None,
        completion_evidence: Vec::new(),
        evidence_refs: vec!["git:commit:test".to_string()],
    }
}

#[test]
fn pending_delivery_binds_submitter_target_and_admitted_envelope() {
    assert!(validate_agent_workload_delivery(&delivery()).is_empty());
}

#[test]
fn accepted_delivery_requires_content_addressed_durable_read_back() {
    let mut delivery = delivery();
    delivery.status = AgentWorkloadDeliveryStatus::Accepted;
    delivery.accepted_at_ms = Some(2);
    delivery.accepted_record_digest = Some("not-a-digest".to_string());

    assert_eq!(
        validate_agent_workload_delivery(&delivery),
        vec!["agent_workload_delivery_acceptance_invalid"]
    );
}

fn publication() -> (AgentWorkloadDelivery, AgentWorkloadCompletionPublication) {
    let mut accepted = delivery();
    accepted = accept_agent_workload_delivery(
        &accepted,
        &AgentWorkloadDeliveryAcceptance {
            consumer_instance_id: "builder-incarnation-a".to_string(),
            record_digest: format!("sha256:{}", "1".repeat(64)),
            requested_at_ms: 2,
            lease_ttl_ms: 10,
            prior_lease_id: None,
            prior_fencing_token: None,
        },
    )
    .unwrap();
    let evidence = build_evidence_envelope(
        "build-workload:delivery-test:canonical_gate_report",
        "cynic-node/build-scheduler:cynic-builder",
        3,
        vec!["test".to_string()],
        &serde_json::json!({"status": "passed"}),
    )
    .unwrap();
    let completion = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: accepted.envelope.delegation.delegation_id.clone(),
        node_id: accepted.target_node_id.clone(),
        agent_id: accepted.submitted_by.clone(),
        parent_agent_id: None,
        status: AgentCompletionStatus::Completed,
        evidence_refs: vec![AgentEvidenceRef {
            kind: "canonical_gate_report".to_string(),
            subject: accepted.envelope.workload_id.clone(),
            digest: evidence.digest.clone(),
            lineage: None,
        }],
        unknowns: Vec::new(),
        consumed_budget: AgentBudget {
            context_tokens: 0,
            actions: 0,
            children: 0,
            wall_time_ms: 1,
            cpu_ms: None,
        },
    };
    let record = AgentWorkloadRecord {
        schema: AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        envelope: accepted.envelope.clone(),
        state: AgentWorkloadState::Completed,
        claim: None,
        last_fencing_token: 0,
        cancellation_generation: 0,
        consumed_actions: 0,
        completion: Some(completion),
    };
    let durable = DurableAgentWorkloadRecord {
        schema: DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        revision: 2,
        previous_record_digest: Some(format!("sha256:{}", "2".repeat(64))),
        record_digest: digest_json(&record).unwrap(),
        record,
        committed_at_ms: 3,
    };
    let prior = accepted.acceptance_lease.clone().unwrap();
    accepted = accept_agent_workload_delivery(
        &accepted,
        &AgentWorkloadDeliveryAcceptance {
            consumer_instance_id: prior.consumer_instance_id,
            record_digest: durable.record_digest.clone(),
            requested_at_ms: 3,
            lease_ttl_ms: 10,
            prior_lease_id: Some(prior.lease_id),
            prior_fencing_token: Some(prior.fencing_token),
        },
    )
    .unwrap();
    let publication = AgentWorkloadCompletionPublication {
        schema: AGENT_WORKLOAD_COMPLETION_PUBLICATION_SCHEMA.to_string(),
        node_id: accepted.target_node_id.clone(),
        workload_id: accepted.envelope.workload_id.clone(),
        published_at_ms: 4,
        delivery_lease: accepted.acceptance_lease.clone().unwrap(),
        durable_record: durable,
        evidence: vec![evidence],
    };
    (accepted, publication)
}

#[test]
fn terminal_delivery_is_derived_from_exact_durable_completion_and_evidence() {
    let (accepted, publication) = publication();

    let terminal = complete_agent_workload_delivery(&accepted, &publication).unwrap();

    assert_eq!(terminal.status, AgentWorkloadDeliveryStatus::Terminal);
    assert_eq!(
        terminal.terminal_record_digest.as_deref(),
        Some(publication.durable_record.record_digest.as_str())
    );
    assert!(validate_agent_workload_delivery(&terminal).is_empty());
}

#[test]
fn altered_completion_evidence_is_rejected() {
    let (accepted, mut publication) = publication();
    publication.evidence[0].payload = serde_json::json!({"status": "forged"});

    assert_eq!(
        validate_agent_workload_completion_publication(&accepted, &publication),
        vec!["agent_workload_completion_evidence_digest_invalid"]
    );
}

#[test]
fn expired_acceptance_is_recoverable_and_fenced() {
    let first = accept_agent_workload_delivery(
        &delivery(),
        &AgentWorkloadDeliveryAcceptance {
            consumer_instance_id: "builder-incarnation-a".to_string(),
            record_digest: format!("sha256:{}", "1".repeat(64)),
            requested_at_ms: 2,
            lease_ttl_ms: 10,
            prior_lease_id: None,
            prior_fencing_token: None,
        },
    )
    .unwrap();
    assert!(!agent_workload_delivery_available(&first, 11));
    assert!(agent_workload_delivery_available(&first, 12));

    let reclaimed = accept_agent_workload_delivery(
        &first,
        &AgentWorkloadDeliveryAcceptance {
            consumer_instance_id: "builder-incarnation-b".to_string(),
            record_digest: format!("sha256:{}", "1".repeat(64)),
            requested_at_ms: 12,
            lease_ttl_ms: 10,
            prior_lease_id: None,
            prior_fencing_token: None,
        },
    )
    .unwrap();
    assert_eq!(reclaimed.acceptance_lease.unwrap().fencing_token, 2);
}

#[test]
fn active_acceptance_only_renews_with_exact_fence() {
    let first = accept_agent_workload_delivery(
        &delivery(),
        &AgentWorkloadDeliveryAcceptance {
            consumer_instance_id: "builder-incarnation-a".to_string(),
            record_digest: format!("sha256:{}", "1".repeat(64)),
            requested_at_ms: 2,
            lease_ttl_ms: 10,
            prior_lease_id: None,
            prior_fencing_token: None,
        },
    )
    .unwrap();
    let lease = first.acceptance_lease.as_ref().unwrap();
    let renewed = accept_agent_workload_delivery(
        &first,
        &AgentWorkloadDeliveryAcceptance {
            consumer_instance_id: lease.consumer_instance_id.clone(),
            record_digest: lease.record_digest.clone(),
            requested_at_ms: 5,
            lease_ttl_ms: 10,
            prior_lease_id: Some(lease.lease_id.clone()),
            prior_fencing_token: Some(lease.fencing_token),
        },
    )
    .unwrap();
    assert_eq!(renewed.acceptance_lease.unwrap().fencing_token, 1);

    let conflict = accept_agent_workload_delivery(
        &first,
        &AgentWorkloadDeliveryAcceptance {
            consumer_instance_id: "builder-incarnation-b".to_string(),
            record_digest: format!("sha256:{}", "1".repeat(64)),
            requested_at_ms: 5,
            lease_ttl_ms: 10,
            prior_lease_id: None,
            prior_fencing_token: None,
        },
    )
    .unwrap_err();
    assert!(conflict.contains(&"agent_workload_delivery_lease_active".to_string()));
}

#[test]
fn submission_read_back_unwraps_kernel_persisted_envelope() {
    let sent = delivery();
    let body = serde_json::to_string(&KernelDeliveryEnvelope {
        schema: "cynic.agent_workload_delivery_response.v1".to_string(),
        status: "persisted".to_string(),
        value: sent.clone(),
    })
    .unwrap();
    assert!(validate_agent_workload_submission_read_back(&sent, &body).is_empty());
}

#[test]
fn submission_read_back_rejects_wrong_envelope_status() {
    let sent = delivery();
    let body = serde_json::to_string(&KernelDeliveryEnvelope {
        schema: "cynic.agent_workload_delivery_response.v1".to_string(),
        status: "observed".to_string(),
        value: sent.clone(),
    })
    .unwrap();
    assert_eq!(
        validate_agent_workload_submission_read_back(&sent, &body),
        vec!["agent_workload_submission_read_back_unexpected_status:observed".to_string()]
    );
}

#[test]
fn observation_read_back_unwraps_kernel_observed_envelope() {
    let sent = delivery();
    let body = serde_json::to_value(&KernelDeliveryEnvelope {
        schema: "cynic.agent_workload_delivery_response.v1".to_string(),
        status: "observed".to_string(),
        value: sent.clone(),
    })
    .unwrap();
    assert_eq!(
        parse_agent_workload_observation_read_back(&body).unwrap(),
        sent
    );
}
