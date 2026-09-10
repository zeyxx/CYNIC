use cynic_protocol::RELEASE_BINARY_ARTIFACT_READ_BACK_SCHEMA;

use std::{collections::BTreeMap, time::Duration};

use cynic_host::{
    adapters::local_evidence_store::LocalContentAddressedEvidenceStore,
    ports::evidence_store::ContentAddressedEvidenceStorePort,
};
use cynic_protocol::{
    AgentEvidenceRef, AgentWorkloadEnvelope, RUNTIME_RELEASE_PUBLICATION_MANIFEST_SCHEMA,
    RuntimeReleasePublicationManifest, SIGNED_RELEASE_BASKET_REQUEST_SCHEMA,
    SignedReleaseBasketRequest, build_agent_request_from_workload, build_evidence_envelope,
    plan_signed_release_basket,
};

use crate::{
    adapters::{
        http_release_artifact_publication::HttpReleaseArtifactPublication,
        local_release_basket_packaging::LocalReleaseBasketPackaging,
        ssh_release_basket_signing::SshReleaseBasketSigning,
    },
    config::ReleasePublicationConfig,
    ports::{
        build_scheduler_port::BuildWorkspaceReadBack,
        release_publication_port::{
            ReleaseArtifactPublicationPort, ReleaseBasketPackagingPort, ReleaseBasketSigningPort,
        },
    },
};

pub(super) async fn publish(
    node_id: &str,
    envelope: &AgentWorkloadEnvelope,
    workspace: &BuildWorkspaceReadBack,
    evidence_refs: &[AgentEvidenceRef],
    config: Option<&ReleasePublicationConfig>,
    observed_at_ms: u64,
    provenance: &[String],
    evidence_store: &LocalContentAddressedEvidenceStore,
) -> Result<Option<AgentEvidenceRef>, String> {
    if cynic_protocol::canonical_build_action_from_argv(&envelope.action.argv)
        != Some("make:build-release")
    {
        return Ok(None);
    }
    let request = build_agent_request_from_workload(envelope).map_err(|items| items.join(","))?;
    let publication_intent = request
        .release_publication
        .ok_or_else(|| "release_publication_intent_missing".to_string())?;
    let config = config.ok_or_else(|| "release_publication_config_unavailable".to_string())?;
    let registry_digest = crate::adapters::local_release_basket_packaging::digest_file(
        &workspace.workspace_root.join("infra/registry.json"),
        config.max_bytes,
    )?
    .0;
    let packaging = LocalReleaseBasketPackaging::new(config.work_root.clone(), config.max_bytes)?;
    let signing = SshReleaseBasketSigning::new(
        config.signing_key_path.clone(),
        config.signer_identity.clone(),
        config.signature_namespace.clone(),
        config.max_bytes,
    )?;
    let artifact_store = HttpReleaseArtifactPublication::new(
        &config.artifact_base_url,
        &config.authentication_kind,
        config.credential_path.clone(),
        config.max_bytes,
        Duration::from_secs(config.timeout_secs),
    )?;
    publish_with_ports(
        node_id,
        envelope,
        workspace,
        evidence_refs,
        registry_digest,
        publication_intent,
        observed_at_ms,
        provenance,
        evidence_store,
        &packaging,
        &signing,
        &artifact_store,
    )
    .await
    .map(Some)
}

#[allow(clippy::too_many_arguments)]
async fn publish_with_ports<P, S, A>(
    node_id: &str,
    envelope: &AgentWorkloadEnvelope,
    workspace: &BuildWorkspaceReadBack,
    evidence_refs: &[AgentEvidenceRef],
    registry_digest: String,
    publication_intent: cynic_protocol::BuildReleasePublicationIntent,
    observed_at_ms: u64,
    provenance: &[String],
    evidence_store: &dyn ContentAddressedEvidenceStorePort,
    packaging: &P,
    signing: &S,
    artifact_store: &A,
) -> Result<AgentEvidenceRef, String>
where
    P: ReleaseBasketPackagingPort,
    S: ReleaseBasketSigningPort,
    A: ReleaseArtifactPublicationPort,
{
    let build_proof_ref = evidence_refs
        .iter()
        .find(|reference| reference.kind == "release_binary_artifact_read_back")
        .ok_or_else(|| "release_publication_build_proof_reference_missing".to_string())?;
    let build_proof = evidence_store
        .read(&build_proof_ref.digest)?
        .ok_or_else(|| "release_publication_build_proof_not_retained".to_string())?;
    if build_proof.digest != build_proof_ref.digest
        || build_proof.payload["schema"].as_str() != Some(RELEASE_BINARY_ARTIFACT_READ_BACK_SCHEMA)
        || build_proof.payload["source_commit"].as_str() != Some(workspace.source_commit.as_str())
    {
        return Err("release_publication_build_proof_invalid".to_string());
    }
    let artifacts = serde_json::from_value::<BTreeMap<String, String>>(
        build_proof.payload["artifacts"].clone(),
    )
    .map_err(|_| "release_publication_artifact_proof_invalid".to_string())?;
    let request = SignedReleaseBasketRequest {
        schema: SIGNED_RELEASE_BASKET_REQUEST_SCHEMA.to_string(),
        node_id: node_id.to_string(),
        workload_id: envelope.workload_id.clone(),
        source_commit: workspace.source_commit.clone(),
        build_proof_digest: build_proof_ref.digest.clone(),
        registry_digest,
        publication: publication_intent,
        artifacts,
    };
    let plan = plan_signed_release_basket(&request);
    if plan.status != "planned" {
        return Err(format!(
            "release_publication_plan_blocked:{}",
            plan.blockers.join(",")
        ));
    }
    let basket = packaging
        .package(&plan, &workspace.workspace_root, &envelope.workload_id)
        .await?;
    let signature = signing.sign(&basket).await?;
    let targets = artifact_store.targets(&plan.marker.release_id)?;
    let manifest = RuntimeReleasePublicationManifest {
        schema: RUNTIME_RELEASE_PUBLICATION_MANIFEST_SCHEMA.to_string(),
        release_id: plan.marker.release_id.clone(),
        release_epoch: plan.marker.release_epoch,
        source_commit: plan.marker.source_commit.clone(),
        build_proof_digest: plan.marker.build_proof_digest.clone(),
        registry_digest: plan.marker.registry_digest.clone(),
        basket_digest: basket.digest.clone(),
        basket_source: targets.basket.clone(),
        detached_signature_digest: signature.digest.clone(),
        detached_signature_source: targets.detached_signature.clone(),
        signer_identity: signature.signer_identity.clone(),
        signature_namespace: signature.namespace.clone(),
        // The pipeline has no SBOM or capability-inventory stage: report the
        // absence as digest-covered unknowns instead of implying coverage.
        residual_unknowns: vec![
            "release_sbom_unobserved".to_string(),
            "release_capability_inventory_unobserved".to_string(),
        ],
    };
    let read_back = artifact_store
        .publish(
            node_id,
            &envelope.workload_id,
            &basket,
            &signature,
            &manifest,
            &targets,
            observed_at_ms,
        )
        .await?;
    let mut publication_provenance = provenance.to_vec();
    publication_provenance.push(build_proof_ref.digest.clone());
    let evidence = build_evidence_envelope(
        &format!(
            "build-workload:{}:signed_release_basket_read_back",
            envelope.workload_id
        ),
        &format!("cynic-node/release-publication:{node_id}"),
        u128::from(observed_at_ms),
        publication_provenance,
        &read_back,
    )?;
    evidence_store.persist(&evidence)?;
    if evidence_store.read(&evidence.digest)? != Some(evidence.clone()) {
        return Err("release_publication_evidence_read_back_mismatch".to_string());
    }
    Ok(AgentEvidenceRef {
        kind: "signed_release_basket_read_back".to_string(),
        subject: envelope.workload_id.clone(),
        digest: evidence.digest,
        lineage: None,
    })
}
