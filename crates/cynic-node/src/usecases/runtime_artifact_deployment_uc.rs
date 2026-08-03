use std::path::Path;

use cynic_host::runtime_seed::RuntimeSeedApplyReport;
use cynic_protocol::{RuntimeSeedAction, RuntimeSeedPlan, RuntimeSeedRequest};
use serde::Serialize;

use crate::ports::{
    runtime_artifact_transport_port::{
        RuntimeArtifactTransferReceipt, RuntimeArtifactTransportPort,
    },
    runtime_seed_actuation_port::RuntimeSeedActuationPort,
};

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeArtifactDeploymentReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub release_id: String,
    pub mutates: bool,
    pub transfers: Vec<RuntimeArtifactTransferReceipt>,
    pub activation: Option<RuntimeSeedApplyReport>,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
    pub rollback_intent: &'static str,
}

pub async fn reconcile_runtime_artifact_deployment<T, A>(
    request: &RuntimeSeedRequest,
    plan: &RuntimeSeedPlan,
    transport: &T,
    actuation: &A,
) -> RuntimeArtifactDeploymentReport
where
    T: RuntimeArtifactTransportPort,
    A: RuntimeSeedActuationPort,
{
    if plan.status != "planned" {
        return report(
            request,
            false,
            Vec::new(),
            None,
            Vec::new(),
            vec!["runtime_seed_plan_not_admitted".to_string()],
        );
    }

    let mut transfers = Vec::new();
    if plan
        .actions
        .contains(&RuntimeSeedAction::StageReleaseArtifacts)
    {
        match transport
            .stage(
                &request.basket_source,
                Path::new(&request.basket_path),
                Some(&request.basket_digest),
            )
            .await
        {
            Ok(receipt) if receipt.status == "proved" => transfers.push(receipt),
            Ok(receipt) => {
                let blockers = receipt.blockers.clone();
                transfers.push(receipt);
                return report(
                    request,
                    transfers.iter().any(|receipt| receipt.mutates),
                    transfers,
                    None,
                    Vec::new(),
                    blockers,
                );
            }
            Err(error) => {
                return report(
                    request,
                    false,
                    transfers,
                    None,
                    Vec::new(),
                    vec![format!("runtime_basket_transport_failed:{error}")],
                );
            }
        }
        match transport
            .stage(
                &request.detached_signature_source,
                Path::new(&request.detached_signature_path),
                None,
            )
            .await
        {
            Ok(receipt) if receipt.status == "proved" => transfers.push(receipt),
            Ok(receipt) => {
                let blockers = receipt.blockers.clone();
                transfers.push(receipt);
                return report(
                    request,
                    transfers.iter().any(|receipt| receipt.mutates),
                    transfers,
                    None,
                    Vec::new(),
                    blockers,
                );
            }
            Err(error) => {
                return report(
                    request,
                    transfers.iter().any(|receipt| receipt.mutates),
                    transfers,
                    None,
                    Vec::new(),
                    vec![format!("runtime_signature_transport_failed:{error}")],
                );
            }
        }
    }

    let activation = actuation.apply(request);
    let mut blockers = Vec::new();
    if activation.status != "proved" {
        blockers.push(format!(
            "runtime_seed_activation_not_proved:{}",
            activation.status
        ));
        if let Some(error) = activation.error.as_deref() {
            blockers.push(error.to_string());
        }
    }
    let mut evidence_refs = transfers
        .iter()
        .map(|receipt| receipt.evidence_ref.clone())
        .collect::<Vec<_>>();
    match actuation.persist(request, &activation) {
        Ok(path) => evidence_refs.push(format!("file:{}", path.display())),
        Err(error) => blockers.push(format!("runtime_seed_evidence_persist_failed:{error}")),
    }
    evidence_refs.sort();
    evidence_refs.dedup();
    let mutates = transfers.iter().any(|receipt| receipt.mutates)
        || activation.activated
        || activation.registry_activated;

    report(
        request,
        mutates,
        transfers,
        Some(activation),
        evidence_refs,
        blockers,
    )
}

fn report(
    request: &RuntimeSeedRequest,
    mutates: bool,
    transfers: Vec<RuntimeArtifactTransferReceipt>,
    activation: Option<RuntimeSeedApplyReport>,
    mut evidence_refs: Vec<String>,
    blockers: Vec<String>,
) -> RuntimeArtifactDeploymentReport {
    evidence_refs.extend(transfers.iter().map(|receipt| receipt.evidence_ref.clone()));
    evidence_refs.sort();
    evidence_refs.dedup();
    RuntimeArtifactDeploymentReport {
        schema: "cynic.runtime_artifact.deployment_report.v1",
        status: if blockers.is_empty() {
            "proved"
        } else {
            "blocked"
        },
        release_id: request.release_id.clone(),
        mutates,
        transfers,
        activation,
        evidence_refs,
        blockers,
        rollback_intent: "retain transfer evidence and restore the previous runtime slot when activation or postflight fails",
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, path::PathBuf};

    use cynic_host::runtime_seed::RuntimeSeedApplyReport;
    use cynic_protocol::{
        RUNTIME_SEED_OBSERVATION_SCHEMA, RUNTIME_SEED_PLAN_SCHEMA, RUNTIME_SEED_REQUEST_SCHEMA,
        RuntimeArtifactSource, RuntimeArtifactTransportKind, RuntimeSeedObservation,
        SecurityEvidenceState,
    };

    use super::*;

    struct Transport(RefCell<Vec<String>>);

    impl RuntimeArtifactTransportPort for Transport {
        async fn stage(
            &self,
            source: &RuntimeArtifactSource,
            destination: &Path,
            expected_digest: Option<&str>,
        ) -> Result<RuntimeArtifactTransferReceipt, String> {
            self.0.borrow_mut().push(source.locator.clone());
            Ok(RuntimeArtifactTransferReceipt {
                schema: "cynic.runtime_artifact.transfer_receipt.v1",
                status: "proved",
                source: source.clone(),
                destination: destination.display().to_string(),
                expected_digest: expected_digest.map(str::to_string),
                observed_digest: format!("sha256:{}", "a".repeat(64)),
                bytes_written: 10,
                mutates: true,
                evidence_ref: format!("sha256:{}", "a".repeat(64)),
                blockers: Vec::new(),
            })
        }
    }

    struct Actuation;

    impl RuntimeSeedActuationPort for Actuation {
        fn apply(&self, request: &RuntimeSeedRequest) -> RuntimeSeedApplyReport {
            RuntimeSeedApplyReport {
                schema: "cynic.host.runtime_seed.apply_report.v3",
                status: "proved".to_string(),
                before: observation(request),
                after: Some(observation(request)),
                activated: true,
                registry_activated: true,
                rolled_back: false,
                error: None,
            }
        }

        fn persist(
            &self,
            _request: &RuntimeSeedRequest,
            _report: &RuntimeSeedApplyReport,
        ) -> Result<PathBuf, String> {
            Ok("/var/lib/cynic/runtime/evidence/release.json".into())
        }
    }

    fn request() -> RuntimeSeedRequest {
        RuntimeSeedRequest {
            schema: RUNTIME_SEED_REQUEST_SCHEMA.to_string(),
            node_id: "node-a".to_string(),
            release_id: "release-2".to_string(),
            release_epoch: 2,
            source_commit: "c".repeat(40),
            build_proof_digest: format!("sha256:{}", "d".repeat(64)),
            basket_digest: format!("sha256:{}", "a".repeat(64)),
            registry_digest: format!("sha256:{}", "e".repeat(64)),
            basket_source: RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::Https,
                locator: "https://storage.synapse.cynic/release-2.tar".to_string(),
            },
            basket_path: "/var/lib/cynic/inbox/release-2.tar".to_string(),
            detached_signature_source: RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::Https,
                locator: "https://storage.synapse.cynic/release-2.tar.sig".to_string(),
            },
            detached_signature_path: "/var/lib/cynic/inbox/release-2.tar.sig".to_string(),
            allowed_signers_path: "/etc/cynic/runtime-allowed-signers".to_string(),
            registry_path: "/etc/cynic/registry.json".to_string(),
            signer_identity: "release-authority".to_string(),
            signature_namespace: "cynic-runtime-seed".to_string(),
            required_seed_contract_revision: 4,
            install_root: "/var/lib/cynic/runtime".to_string(),
            staged_slot: "releases/release-2".to_string(),
            current_link: "current".to_string(),
            previous_link: "previous".to_string(),
            postflight_argv: vec!["/bin/true".to_string()],
        }
    }

    fn observation(request: &RuntimeSeedRequest) -> RuntimeSeedObservation {
        RuntimeSeedObservation {
            schema: RUNTIME_SEED_OBSERVATION_SCHEMA.to_string(),
            observed_at_unix_ms: 1,
            node_id: request.node_id.clone(),
            seed_contract_revision: Some(4),
            trust_root: SecurityEvidenceState::Proven,
            signature: SecurityEvidenceState::Proven,
            basket_present: true,
            detached_signature_present: true,
            basket_digest_read_back: Some(request.basket_digest.clone()),
            current_release_id: Some(request.release_id.clone()),
            current_release_epoch: Some(request.release_epoch),
            current_source_commit: Some(request.source_commit.clone()),
            current_build_proof_digest: Some(request.build_proof_digest.clone()),
            current_basket_digest: Some(request.basket_digest.clone()),
            current_registry_digest: Some(request.registry_digest.clone()),
            installed_registry_digest: Some(request.registry_digest.clone()),
            previous_release_id: None,
            slot_layout: SecurityEvidenceState::Proven,
            postflight: SecurityEvidenceState::Proven,
            evidence_refs: Vec::new(),
            unknowns: Vec::new(),
        }
    }

    #[tokio::test]
    async fn stages_basket_and_signature_before_node_owned_activation() {
        let request = request();
        let plan = RuntimeSeedPlan {
            schema: RUNTIME_SEED_PLAN_SCHEMA.to_string(),
            status: "planned".to_string(),
            node_id: request.node_id.clone(),
            release_id: request.release_id.clone(),
            release_epoch: request.release_epoch,
            source_commit: request.source_commit.clone(),
            build_proof_digest: request.build_proof_digest.clone(),
            actions: vec![
                RuntimeSeedAction::StageReleaseArtifacts,
                RuntimeSeedAction::VerifySignatureAndDigest,
                RuntimeSeedAction::ActivateStagedSlot,
                RuntimeSeedAction::RunPostflight,
                RuntimeSeedAction::CommitPreviousPointer,
            ],
            blockers: Vec::new(),
            unknowns: Vec::new(),
            evidence_refs: Vec::new(),
            rollback_intent: "restore previous".to_string(),
        };
        let transport = Transport(RefCell::new(Vec::new()));

        let report =
            reconcile_runtime_artifact_deployment(&request, &plan, &transport, &Actuation).await;

        assert_eq!(report.status, "proved");
        assert_eq!(report.transfers.len(), 2);
        assert_eq!(transport.0.borrow().len(), 2);
        assert!(
            report
                .activation
                .as_ref()
                .is_some_and(|value| value.activated)
        );
    }
}
