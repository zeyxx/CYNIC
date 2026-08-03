use serde::{Deserialize, Serialize};

use crate::{
    AgentCompletionStatus, AgentWorkloadDelivery, AgentWorkloadDeliveryStatus, NodeDesiredState,
    SignedReleaseBasketReadBack, build_agent_request_from_workload,
    build_source_reference_from_workload, canonical_build_action_from_argv, digest_json,
    validate_agent_workload_delivery, validate_node_desired_state,
    validate_signed_release_basket_read_back,
};

pub const RUNTIME_RELEASE_DECLARATION_SCHEMA: &str = "cynic.runtime_release_declaration.v1";
pub const RUNTIME_RELEASE_ADMISSION_SCHEMA: &str = "cynic.runtime_release_admission.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeReleaseDeclaration {
    pub schema: String,
    pub requested_by: String,
    pub workload_id: String,
    pub desired_state: NodeDesiredState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeReleaseAdmission {
    pub schema: String,
    pub admitted: bool,
    pub workload_id: String,
    pub desired_state_digest: Option<String>,
    pub terminal_record_digest: Option<String>,
    pub build_proof_digest: String,
    pub publication_proof_digest: Option<String>,
    #[serde(default)]
    pub violations: Vec<String>,
}

pub fn runtime_release_policy_digest(desired: &NodeDesiredState) -> Result<String, String> {
    digest_json(&serde_json::json!({
        "schema": desired.schema,
        "node_id": desired.node_id,
        "role": desired.role,
        "generation": desired.generation,
        "release": desired.release,
        "required_capabilities": desired.required_capabilities,
        "required_services": desired.required_services,
        "paused": desired.paused,
    }))
    .map_err(|error| format!("failed to digest runtime release policy: {error}"))
}

pub fn admit_runtime_release(
    declaration: &RuntimeReleaseDeclaration,
    delivery: &AgentWorkloadDelivery,
) -> RuntimeReleaseAdmission {
    let mut violations = Vec::new();
    if declaration.schema != RUNTIME_RELEASE_DECLARATION_SCHEMA {
        violations.push("runtime_release_declaration_schema_mismatch".to_string());
    }
    if declaration.requested_by.is_empty()
        || declaration.requested_by != delivery.submitted_by
        || declaration.workload_id != delivery.envelope.workload_id
    {
        violations.push("runtime_release_declaration_identity_mismatch".to_string());
    }
    violations.extend(validate_node_desired_state(&declaration.desired_state));
    match runtime_release_policy_digest(&declaration.desired_state) {
        Ok(digest) if digest == declaration.desired_state.policy_digest => {}
        _ => violations.push("runtime_release_policy_digest_mismatch".to_string()),
    }
    violations.extend(validate_agent_workload_delivery(delivery));
    if delivery.status != AgentWorkloadDeliveryStatus::Terminal {
        violations.push("runtime_release_workload_not_terminal".to_string());
    }
    if canonical_build_action_from_argv(&delivery.envelope.action.argv)
        != Some("make:build-release")
    {
        violations.push("runtime_release_workload_action_mismatch".to_string());
    }
    match build_source_reference_from_workload(&delivery.envelope) {
        Ok(source) if source.revision == declaration.desired_state.release.source_commit => {}
        _ => violations.push("runtime_release_source_commit_mismatch".to_string()),
    }
    let completion = delivery.completion.as_ref();
    if completion.map(|completion| completion.status) != Some(AgentCompletionStatus::Completed) {
        violations.push("runtime_release_build_not_completed".to_string());
    }
    let proof_digest = &declaration.desired_state.release.build_proof_digest;
    let proof_ref = completion.and_then(|completion| {
        completion.evidence_refs.iter().find(|reference| {
            reference.kind == "release_binary_artifact_read_back"
                && reference.subject == declaration.workload_id
                && reference.digest == *proof_digest
        })
    });
    if proof_ref.is_none() {
        violations.push("runtime_release_build_proof_reference_missing".to_string());
    }
    let proof = delivery
        .completion_evidence
        .iter()
        .find(|evidence| evidence.digest == *proof_digest);
    match proof {
        Some(proof)
            if valid_release_proof_payload(
                &proof.payload,
                &declaration.desired_state.release.source_commit,
            ) => {}
        Some(_) => violations.push("runtime_release_build_proof_payload_invalid".to_string()),
        None => violations.push("runtime_release_build_proof_not_retained".to_string()),
    }
    match build_agent_request_from_workload(&delivery.envelope) {
        Ok(request)
            if request.release_publication.as_ref().is_some_and(|intent| {
                intent.release_id == declaration.desired_state.release.release_id
                    && intent.release_epoch == declaration.desired_state.release.release_epoch
            }) => {}
        _ => violations.push("runtime_release_publication_intent_mismatch".to_string()),
    }
    let publication_ref = completion.and_then(|completion| {
        completion.evidence_refs.iter().find(|reference| {
            reference.kind == "signed_release_basket_read_back"
                && reference.subject == declaration.workload_id
        })
    });
    let publication = publication_ref.and_then(|reference| {
        delivery
            .completion_evidence
            .iter()
            .find(|evidence| evidence.digest == reference.digest)
    });
    match publication.and_then(|evidence| {
        serde_json::from_value::<SignedReleaseBasketReadBack>(evidence.payload.clone()).ok()
    }) {
        Some(read_back)
            if validate_signed_release_basket_read_back(&read_back).is_empty()
                && release_read_back_matches_desired(
                    &read_back,
                    &declaration.desired_state,
                    &declaration.workload_id,
                    &delivery.target_node_id,
                ) => {}
        Some(_) => violations.push("runtime_release_publication_read_back_mismatch".to_string()),
        None if publication_ref.is_some() => {
            violations.push("runtime_release_publication_read_back_invalid".to_string())
        }
        None => violations.push("runtime_release_publication_proof_missing".to_string()),
    }
    violations.sort();
    violations.dedup();
    RuntimeReleaseAdmission {
        schema: RUNTIME_RELEASE_ADMISSION_SCHEMA.to_string(),
        admitted: violations.is_empty(),
        workload_id: declaration.workload_id.clone(),
        desired_state_digest: digest_json(&declaration.desired_state).ok(),
        terminal_record_digest: delivery.terminal_record_digest.clone(),
        build_proof_digest: proof_digest.clone(),
        publication_proof_digest: publication_ref.map(|reference| reference.digest.clone()),
        violations,
    }
}

fn release_read_back_matches_desired(
    read_back: &SignedReleaseBasketReadBack,
    desired: &NodeDesiredState,
    workload_id: &str,
    builder_node_id: &str,
) -> bool {
    let release = &desired.release;
    let manifest = &read_back.manifest;
    read_back.node_id == builder_node_id
        && read_back.workload_id == workload_id
        && manifest.release_id == release.release_id
        && manifest.release_epoch == release.release_epoch
        && manifest.source_commit == release.source_commit
        && manifest.build_proof_digest == release.build_proof_digest
        && manifest.registry_digest == release.registry_digest
        && manifest.basket_digest == release.basket_digest
        && manifest.basket_source == release.basket_source
        && manifest.detached_signature_source == release.detached_signature_source
        && manifest.signer_identity == release.signer_identity
        && manifest.signature_namespace == release.signature_namespace
}

fn valid_release_proof_payload(payload: &serde_json::Value, source_commit: &str) -> bool {
    if payload["schema"].as_str() != Some("cynic.build.release_binary_artifact_read_back.v1")
        || payload["source_commit"].as_str() != Some(source_commit)
    {
        return false;
    }
    let Some(artifacts) = payload["artifacts"].as_object() else {
        return false;
    };
    ["cynic", "cynic-node", "cynic-kernel", "cynic-mcp"]
        .iter()
        .all(|name| {
            artifacts
                .get(*name)
                .and_then(serde_json::Value::as_str)
                .is_some_and(valid_sha256)
        })
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AGENT_COMPLETION_ENVELOPE_SCHEMA, AGENT_WORKLOAD_DELIVERY_SCHEMA,
        AGENT_WORKLOAD_RECORD_SCHEMA, AgentBudget, AgentCompletionEnvelope, AgentEvidenceRef,
        AgentWorkloadRecord, AgentWorkloadState, BUILD_AGENT_REQUEST_SCHEMA,
        BUILD_SOURCE_REFERENCE_SCHEMA, BuildAgentRequest, BuildSourceReference,
        DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA, DurableAgentWorkloadRecord,
        RUNTIME_SEED_REQUEST_SCHEMA, RuntimeArtifactSource, RuntimeArtifactTransportKind,
        RuntimeReleasePublicationManifest, RuntimeSeedRequest,
        SIGNED_RELEASE_BASKET_READ_BACK_SCHEMA, SignedReleaseBasketReadBack,
        build_evidence_envelope, complete_agent_workload_delivery, compose_build_agent_workload,
    };

    fn desired(source_commit: String, proof_digest: String) -> NodeDesiredState {
        let mut desired = NodeDesiredState {
            schema: crate::NODE_DESIRED_STATE_SCHEMA.to_string(),
            node_id: "node-a".to_string(),
            role: "runtime".to_string(),
            generation: 1,
            policy_digest: String::new(),
            release: RuntimeSeedRequest {
                schema: RUNTIME_SEED_REQUEST_SCHEMA.to_string(),
                node_id: "node-a".to_string(),
                release_id: "release-a".to_string(),
                release_epoch: 1,
                source_commit,
                build_proof_digest: proof_digest,
                basket_digest: format!("sha256:{}", "c".repeat(64)),
                registry_digest: format!("sha256:{}", "e".repeat(64)),
                basket_source: RuntimeArtifactSource {
                    transport: RuntimeArtifactTransportKind::Https,
                    locator: "https://artifacts.cynic/release-a.tar".to_string(),
                },
                basket_path: "/var/lib/cynic/inbox/release-a.tar".to_string(),
                detached_signature_source: RuntimeArtifactSource {
                    transport: RuntimeArtifactTransportKind::Https,
                    locator: "https://artifacts.cynic/release-a.sig".to_string(),
                },
                detached_signature_path: "/var/lib/cynic/inbox/release-a.sig".to_string(),
                allowed_signers_path: "/etc/cynic/allowed_signers".to_string(),
                registry_path: "/etc/cynic/registry.json".to_string(),
                signer_identity: "cynic-release".to_string(),
                signature_namespace: "cynic-runtime-seed".to_string(),
                required_seed_contract_revision: 4,
                install_root: "/opt/cynic".to_string(),
                staged_slot: "releases/release-a".to_string(),
                current_link: "current".to_string(),
                previous_link: "previous".to_string(),
                postflight_argv: vec![
                    "/opt/cynic/current/cynic".to_string(),
                    "--version".to_string(),
                ],
            },
            required_capabilities: Vec::new(),
            required_services: Vec::new(),
            paused: false,
        };
        desired.policy_digest = runtime_release_policy_digest(&desired).unwrap();
        desired
    }

    #[test]
    fn policy_digest_binds_the_complete_release_projection() {
        let desired = desired("a".repeat(40), format!("sha256:{}", "b".repeat(64)));
        let digest = runtime_release_policy_digest(&desired).unwrap();
        let mut changed = desired;
        changed.release.release_epoch += 1;

        assert_eq!(digest, changed.policy_digest);
        assert_ne!(
            runtime_release_policy_digest(&changed).unwrap(),
            changed.policy_digest
        );
    }

    fn terminal_release_build() -> (AgentWorkloadDelivery, String) {
        let source_commit = "a".repeat(40);
        let request = BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "release-build-a".to_string(),
            requested_by: "release-agent".to_string(),
            source_commit: source_commit.clone(),
            source: BuildSourceReference {
                schema: BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: "cynic".to_string(),
                transport_uri: "ssh://forgejo@forgejo.synapse.cynic:22/owner/CYNIC.git".to_string(),
                revision: source_commit.clone(),
                authority_ref: "infra/repo-remotes.json:forgejo".to_string(),
                snapshot: None,
            },
            action: "make:build-release".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: true,
            proof_requirements: vec![
                "release_binary_artifact_read_back".to_string(),
                "signed_release_basket_read_back".to_string(),
                "source_commit_read_back".to_string(),
                "build_runner_observation".to_string(),
            ],
            release_publication: Some(crate::BuildReleasePublicationIntent {
                schema: crate::BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA.to_string(),
                release_id: "release-a".to_string(),
                release_epoch: 1,
            }),
            evidence_refs: vec!["git:commit:release-a".to_string()],
        };
        let envelope = compose_build_agent_workload(
            &request,
            "cynic-builder",
            "/var/lib/cynic/source/workspaces",
        )
        .unwrap();
        let workload_id = envelope.workload_id.clone();
        let release_proof = build_evidence_envelope(
            &format!("build-workload:{workload_id}:release_binary_artifact_read_back"),
            "cynic-node/build-scheduler:cynic-builder",
            3,
            vec!["test".to_string()],
            &serde_json::json!({
                "schema": "cynic.build.release_binary_artifact_read_back.v1",
                "source_commit": source_commit,
                "artifacts": {
                    "cynic": format!("sha256:{}", "1".repeat(64)),
                    "cynic-node": format!("sha256:{}", "2".repeat(64)),
                    "cynic-kernel": format!("sha256:{}", "3".repeat(64)),
                    "cynic-mcp": format!("sha256:{}", "4".repeat(64)),
                }
            }),
        )
        .unwrap();
        let publication_manifest = RuntimeReleasePublicationManifest {
            schema: crate::RUNTIME_RELEASE_PUBLICATION_MANIFEST_SCHEMA.to_string(),
            release_id: "release-a".to_string(),
            release_epoch: 1,
            source_commit: source_commit.clone(),
            build_proof_digest: release_proof.digest.clone(),
            registry_digest: format!("sha256:{}", "e".repeat(64)),
            basket_digest: format!("sha256:{}", "c".repeat(64)),
            basket_source: RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::Https,
                locator: "https://artifacts.cynic/release-a.tar".to_string(),
            },
            detached_signature_digest: format!("sha256:{}", "d".repeat(64)),
            detached_signature_source: RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::Https,
                locator: "https://artifacts.cynic/release-a.sig".to_string(),
            },
            signer_identity: "cynic-release".to_string(),
            signature_namespace: crate::RUNTIME_RELEASE_SIGNATURE_NAMESPACE.to_string(),
        };
        let manifest_digest = digest_json(&publication_manifest).unwrap();
        let signed_release = build_evidence_envelope(
            &format!("build-workload:{workload_id}:signed_release_basket_read_back"),
            "cynic-node/release-publication:cynic-builder",
            3,
            vec![release_proof.digest.clone()],
            &SignedReleaseBasketReadBack {
                schema: SIGNED_RELEASE_BASKET_READ_BACK_SCHEMA.to_string(),
                node_id: "cynic-builder".to_string(),
                workload_id: workload_id.clone(),
                manifest: publication_manifest,
                manifest_digest,
                manifest_source: RuntimeArtifactSource {
                    transport: RuntimeArtifactTransportKind::Https,
                    locator: "https://artifacts.cynic/release-a.manifest.json".to_string(),
                },
                basket_size_bytes: 1024,
                detached_signature_size_bytes: 256,
                observed_at_ms: 3,
            },
        )
        .unwrap();
        let auxiliary = ["source_commit_read_back", "build_runner_observation"]
            .into_iter()
            .map(|kind| {
                build_evidence_envelope(
                    &format!("build-workload:{workload_id}:{kind}"),
                    "cynic-node/build-scheduler:cynic-builder",
                    3,
                    vec!["test".to_string()],
                    &serde_json::json!({"kind": kind}),
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let mut evidence = vec![release_proof.clone(), signed_release];
        evidence.extend(auxiliary);
        let completion = AgentCompletionEnvelope {
            schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
            delegation_id: envelope.delegation.delegation_id.clone(),
            node_id: envelope.delegation.node_id.clone(),
            agent_id: envelope.delegation.agent_id.clone(),
            parent_agent_id: None,
            status: AgentCompletionStatus::Completed,
            evidence_refs: evidence
                .iter()
                .zip(request.proof_requirements.iter())
                .map(|(evidence, kind)| AgentEvidenceRef {
                    kind: kind.clone(),
                    subject: workload_id.clone(),
                    digest: evidence.digest.clone(),
                    lineage: None,
                })
                .collect(),
            unknowns: Vec::new(),
            consumed_budget: AgentBudget {
                context_tokens: 0,
                actions: 0,
                children: 0,
                wall_time_ms: 1,
            },
        };
        let record = AgentWorkloadRecord {
            schema: AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
            envelope: envelope.clone(),
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
            previous_record_digest: Some(format!("sha256:{}", "5".repeat(64))),
            record_digest: digest_json(&record).unwrap(),
            record,
            committed_at_ms: 3,
        };
        let accepted = AgentWorkloadDelivery {
            schema: AGENT_WORKLOAD_DELIVERY_SCHEMA.to_string(),
            envelope,
            target_node_id: "cynic-builder".to_string(),
            submitted_by: "release-agent".to_string(),
            submitted_at_ms: 1,
            status: AgentWorkloadDeliveryStatus::Accepted,
            accepted_at_ms: Some(2),
            accepted_record_digest: Some(format!("sha256:{}", "6".repeat(64))),
            terminal_at_ms: None,
            terminal_record_digest: None,
            terminal_record: None,
            completion: None,
            completion_evidence: Vec::new(),
            evidence_refs: vec!["git:commit:release-a".to_string()],
        };
        let publication = crate::AgentWorkloadCompletionPublication {
            schema: crate::AGENT_WORKLOAD_COMPLETION_PUBLICATION_SCHEMA.to_string(),
            node_id: "cynic-builder".to_string(),
            workload_id,
            published_at_ms: 4,
            durable_record: durable,
            evidence,
        };
        (
            complete_agent_workload_delivery(&accepted, &publication).unwrap(),
            release_proof.digest,
        )
    }

    #[test]
    fn release_admission_requires_the_retained_exact_build_proof() {
        let (delivery, proof_digest) = terminal_release_build();
        let desired = desired("a".repeat(40), proof_digest);
        let declaration = RuntimeReleaseDeclaration {
            schema: RUNTIME_RELEASE_DECLARATION_SCHEMA.to_string(),
            requested_by: "release-agent".to_string(),
            workload_id: delivery.envelope.workload_id.clone(),
            desired_state: desired,
        };

        assert!(admit_runtime_release(&declaration, &delivery).admitted);

        let mut forged = declaration;
        forged.desired_state.release.build_proof_digest = format!("sha256:{}", "f".repeat(64));
        forged.desired_state.policy_digest =
            runtime_release_policy_digest(&forged.desired_state).unwrap();
        let admission = admit_runtime_release(&forged, &delivery);
        assert!(!admission.admitted);
        assert!(
            admission
                .violations
                .contains(&"runtime_release_build_proof_not_retained".to_string())
        );
    }

    #[test]
    fn release_admission_rejects_a_locator_not_proved_by_publication_read_back() {
        let (delivery, proof_digest) = terminal_release_build();
        let mut desired = desired("a".repeat(40), proof_digest);
        desired.release.basket_source.locator =
            "https://artifacts.cynic/unproved-release-a.tar".to_string();
        desired.policy_digest = runtime_release_policy_digest(&desired).unwrap();
        let declaration = RuntimeReleaseDeclaration {
            schema: RUNTIME_RELEASE_DECLARATION_SCHEMA.to_string(),
            requested_by: "release-agent".to_string(),
            workload_id: delivery.envelope.workload_id.clone(),
            desired_state: desired,
        };

        let admission = admit_runtime_release(&declaration, &delivery);

        assert!(!admission.admitted);
        assert!(
            admission
                .violations
                .contains(&"runtime_release_publication_read_back_mismatch".to_string())
        );
    }
}
