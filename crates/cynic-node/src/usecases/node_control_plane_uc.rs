use cynic_protocol::{
    NodeControlDecision, NodeControlExecutionEvidence, NodeControlPlan, reconcile_node_control,
};
use serde::Serialize;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::ports::node_control_plane_port::{
    NodeControlObservationPort, NodeDesiredStatePullPort, NodeStatusPublishPort,
    SignedNodeDesiredStatePullPort,
};
use cynic_host::plan_verifier::PlanVerifier;
use cynic_protocol::node_control::ControlPlanVerificationStatus;
use crate::ports::runtime_artifact_transport_port::RuntimeArtifactTransportPort;
use crate::ports::runtime_seed_actuation_port::RuntimeSeedActuationPort;

use super::runtime_artifact_deployment_uc::{
    RuntimeArtifactDeploymentReport, reconcile_runtime_artifact_deployment,
};

pub const NODE_CONTROL_TICK_REPORT_SCHEMA: &str = "cynic.node_control_tick_report.v1";

#[derive(Debug, Clone, Serialize)]
pub struct NodeControlTickReport {
    pub schema: &'static str,
    pub status: String,
    pub node_id: String,
    pub desired_generation: Option<u64>,
    pub plan: Option<NodeControlPlan>,
    pub release_deployment: Option<RuntimeArtifactDeploymentReport>,
    pub status_evidence_ref: Option<String>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
}

pub async fn reconcile_node_control_tick<D, O, S, T, A>(
    node_id: &str,
    desired_state: &D,
    observer: &O,
    status_publisher: &S,
    artifact_transport: &T,
    release_actuation: &A,
) -> NodeControlTickReport
where
    D: NodeDesiredStatePullPort,
    O: NodeControlObservationPort,
    S: NodeStatusPublishPort,
    T: RuntimeArtifactTransportPort,
    A: RuntimeSeedActuationPort,
{
    let desired = match desired_state.pull_desired_state(node_id).await {
        Ok(Some(desired)) => desired,
        Ok(None) => {
            return report(
                node_id,
                "waiting_for_desired_state",
                None,
                None,
                None,
                None,
                Vec::new(),
                vec!["node_desired_state_unavailable".to_string()],
            );
        }
        Err(error) => {
            return report(
                node_id,
                "blocked_disconnected",
                None,
                None,
                None,
                None,
                vec![format!("desired_state_pull_failed:{error}")],
                Vec::new(),
            );
        }
    };
    if desired.node_id != node_id {
        return report(
            node_id,
            "blocked",
            Some(desired.generation),
            None,
            None,
            None,
            vec!["desired_state_node_identity_mismatch".to_string()],
            Vec::new(),
        );
    }

    let observation = match observer.observe_node_control(&desired).await {
        Ok(observation) => observation,
        Err(error) => {
            return report(
                node_id,
                "unknown",
                Some(desired.generation),
                None,
                None,
                None,
                Vec::new(),
                vec![format!("node_control_observation_failed:{error}")],
            );
        }
    };
    let mut plan = reconcile_node_control(&desired, &observation);
    let mut release_deployment = None;
    if let NodeControlDecision::ReconcileRelease { plan: release_plan } = plan.decision.clone() {
        let deployment = reconcile_runtime_artifact_deployment(
            &desired.release,
            &release_plan,
            artifact_transport,
            release_actuation,
        )
        .await;
        let execution = NodeControlExecutionEvidence {
            schema: "cynic.node_control.execution_evidence.v1".to_string(),
            operation: "runtime_release_deployment".to_string(),
            status: deployment.status.to_string(),
            mutates: deployment.mutates,
            evidence_refs: deployment.evidence_refs.clone(),
            blockers: deployment.blockers.clone(),
        };
        if deployment.status == "proved" {
            match observer.observe_node_control(&desired).await {
                Ok(after) => plan = reconcile_node_control(&desired, &after),
                Err(error) => {
                    plan.status = "unknown".to_string();
                    plan.unknowns.push(format!(
                        "node_control_post_action_observation_failed:{error}"
                    ));
                }
            }
        }
        plan.evidence_refs.extend(deployment.evidence_refs.clone());
        plan.evidence_refs.sort();
        plan.evidence_refs.dedup();
        plan.execution = Some(execution);
        release_deployment = Some(deployment);
    }
    let desired_generation = Some(plan.desired_generation);
    let plan_status = release_deployment
        .as_ref()
        .filter(|deployment| deployment.status != "proved")
        .map_or_else(|| plan.status.clone(), |_| "blocked".to_string());
    let deployment_blockers = release_deployment
        .as_ref()
        .map(|deployment| deployment.blockers.clone())
        .unwrap_or_default();
    match status_publisher.publish_node_status(&plan).await {
        Ok(evidence_ref) if !evidence_ref.trim().is_empty() => report(
            node_id,
            &plan_status,
            desired_generation,
            Some(plan),
            release_deployment,
            Some(evidence_ref),
            deployment_blockers,
            Vec::new(),
        ),
        Ok(_) => report(
            node_id,
            "unknown",
            desired_generation,
            Some(plan),
            release_deployment,
            None,
            deployment_blockers,
            vec!["node_status_publish_proof_missing".to_string()],
        ),
        Err(error) => report(
            node_id,
            "degraded_disconnected",
            desired_generation,
            Some(plan),
            release_deployment,
            None,
            deployment_blockers
                .into_iter()
                .chain([format!("node_status_publish_failed:{error}")])
                .collect(),
            Vec::new(),
        ),
    }
}

pub async fn reconcile_signed_node_control_tick<D, O, S, T, A>(
    node_id: &str,
    signed_desired_state: &D,
    plan_verifier: &PlanVerifier,
    observed_generation: Option<u64>,
    now_unix_s: u64,
    observer: &O,
    status_publisher: &S,
    artifact_transport: &T,
    release_actuation: &A,
) -> NodeControlTickReport
where
    D: SignedNodeDesiredStatePullPort,
    O: NodeControlObservationPort,
    S: NodeStatusPublishPort,
    T: RuntimeArtifactTransportPort,
    A: RuntimeSeedActuationPort,
{
    let signed_plan = match signed_desired_state.pull_signed_desired_state(node_id).await {
        Ok(Some(signed)) => signed,
        Ok(None) => {
            return report(
                node_id,
                "waiting_for_desired_state",
                None,
                None,
                None,
                None,
                Vec::new(),
                vec!["signed_node_desired_state_unavailable".to_string()],
            );
        }
        Err(error) => {
            return report(
                node_id,
                "blocked_disconnected",
                None,
                None,
                None,
                None,
                vec![format!("signed_desired_state_pull_failed:{error}")],
                Vec::new(),
            );
        }
    };

    let desired = match plan_verifier.verify_and_admit_plan(&signed_plan, observed_generation, now_unix_s) {
        Ok(admitted) => admitted,
        Err(rejection) => {
            let blocker = match rejection {
                ControlPlanVerificationStatus::InvalidSignature => "control_plan_signature_invalid".to_string(),
                ControlPlanVerificationStatus::StaleGeneration { observed, plan } => {
                    format!("control_plan_stale_generation:observed_{observed}_plan_{plan}")
                }
                ControlPlanVerificationStatus::Expired { expires_at_unix_s, now_unix_s } => {
                    format!("control_plan_expired:expires_{expires_at_unix_s}_now_{now_unix_s}")
                }
                ControlPlanVerificationStatus::UntrustedAuthority(auth_id) => {
                    format!("control_plan_untrusted_authority:{auth_id}")
                }
                ControlPlanVerificationStatus::StructuralErrors(errs) => {
                    format!("control_plan_structural_errors:{}", errs.join(","))
                }
            };
            return report(
                node_id,
                "blocked_unadmitted",
                Some(signed_plan.desired_state.generation),
                None,
                None,
                None,
                vec![blocker],
                Vec::new(),
            );
        }
    };

    if desired.node_id != node_id {
        return report(
            node_id,
            "blocked",
            Some(desired.generation),
            None,
            None,
            None,
            vec!["desired_state_node_identity_mismatch".to_string()],
            Vec::new(),
        );
    }

    let observation = match observer.observe_node_control(&desired).await {
        Ok(observation) => observation,
        Err(error) => {
            return report(
                node_id,
                "unknown",
                Some(desired.generation),
                None,
                None,
                None,
                Vec::new(),
                vec![format!("node_control_observation_failed:{error}")],
            );
        }
    };
    let mut plan = reconcile_node_control(&desired, &observation);
    let (release_deployment, deployment_blockers) = match &plan.decision {
        NodeControlDecision::ReconcileRelease { plan: seed_plan } => {
            let report = reconcile_runtime_artifact_deployment(
                &desired.release,
                seed_plan,
                artifact_transport,
                release_actuation,
            )
            .await;
            let blockers = report.blockers.clone();
            (Some(report), blockers)
        }
        _ => (None, Vec::new()),
    };
    if let Some(deployment) = &release_deployment {
        if deployment.status != "converged" {
            plan.blockers.extend(deployment.blockers.clone());
            plan.unknowns.extend(deployment.unknowns.clone());
            plan.decision = match plan.decision {
                NodeControlDecision::ReconcileRelease { .. } => {
                    if !plan.blockers.is_empty() {
                        NodeControlDecision::Blocked {
                            violations: plan.blockers.clone(),
                        }
                    } else {
                        NodeControlDecision::Unknown {
                            missing_evidence: plan.unknowns.clone(),
                        }
                    }
                }
                other => other,
            };
        }
    }
    let desired_generation = Some(desired.generation);
    match status_publisher.publish_node_status(&plan).await {
        Ok(evidence_ref) if !evidence_ref.trim().is_empty() => {
            plan.evidence_refs.push(evidence_ref.clone());
            report(
                node_id,
                &plan.status,
                desired_generation,
                Some(plan),
                release_deployment,
                Some(evidence_ref),
                deployment_blockers,
                Vec::new(),
            )
        }
        Ok(_) => report(
            node_id,
            "unknown",
            desired_generation,
            Some(plan),
            release_deployment,
            None,
            deployment_blockers,
            vec!["node_status_publish_proof_missing".to_string()],
        ),
        Err(error) => report(
            node_id,
            "degraded_disconnected",
            desired_generation,
            Some(plan),
            release_deployment,
            None,
            deployment_blockers
                .into_iter()
                .chain([format!("node_status_publish_failed:{error}")])
                .collect(),
            Vec::new(),
        ),
    }
}

pub async fn run_node_control_loop<D, O, S, T, A>(
    node_id: &str,
    interval: Duration,
    desired_state: &D,
    observer: &O,
    status_publisher: &S,
    artifact_transport: &T,
    release_actuation: &A,
    shutdown: &CancellationToken,
) where
    D: NodeDesiredStatePullPort,
    O: NodeControlObservationPort,
    S: NodeStatusPublishPort,
    T: RuntimeArtifactTransportPort,
    A: RuntimeSeedActuationPort,
{
    let mut ticker = tokio::time::interval(interval.max(Duration::from_secs(1)));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = ticker.tick() => {
                let report = reconcile_node_control_tick(
                    node_id,
                    desired_state,
                    observer,
                    status_publisher,
                    artifact_transport,
                    release_actuation,
                ).await;
                match report.status.as_str() {
                    "converged" => tracing::debug!(node_id, generation = ?report.desired_generation, "node control converged"),
                    _ => tracing::warn!(node_id, status = %report.status, blockers = ?report.blockers, unknowns = ?report.unknowns, "node control remains unconverged"),
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn report(
    node_id: &str,
    status: &str,
    desired_generation: Option<u64>,
    plan: Option<NodeControlPlan>,
    release_deployment: Option<RuntimeArtifactDeploymentReport>,
    status_evidence_ref: Option<String>,
    blockers: Vec<String>,
    unknowns: Vec<String>,
) -> NodeControlTickReport {
    NodeControlTickReport {
        schema: NODE_CONTROL_TICK_REPORT_SCHEMA,
        status: status.to_string(),
        node_id: node_id.to_string(),
        desired_generation,
        plan,
        release_deployment,
        status_evidence_ref,
        blockers,
        unknowns,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        path::{Path, PathBuf},
    };

    use cynic_host::runtime_seed::RuntimeSeedApplyReport;
    use cynic_protocol::{
        NODE_CONTROL_OBSERVATION_SCHEMA, NODE_DESIRED_STATE_SCHEMA, NodeControlConnectionState,
        NodeControlObservation, NodeDesiredState, RUNTIME_SEED_OBSERVATION_SCHEMA,
        RUNTIME_SEED_REQUEST_SCHEMA, RuntimeArtifactSource, RuntimeArtifactTransportKind,
        RuntimeSeedObservation, RuntimeSeedRequest, SecurityEvidenceState,
    };

    use super::*;

    struct Desired(Option<NodeDesiredState>);
    impl NodeDesiredStatePullPort for Desired {
        async fn pull_desired_state(
            &self,
            _node_id: &str,
        ) -> Result<Option<NodeDesiredState>, String> {
            Ok(self.0.clone())
        }
    }

    struct Observer;
    impl NodeControlObservationPort for Observer {
        async fn observe_node_control(
            &self,
            desired: &NodeDesiredState,
        ) -> Result<NodeControlObservation, String> {
            Ok(NodeControlObservation {
                schema: NODE_CONTROL_OBSERVATION_SCHEMA.to_string(),
                node_id: desired.node_id.clone(),
                observed_at_unix_ms: 1,
                observed_generation: Some(desired.generation),
                enrollment: SecurityEvidenceState::Proven,
                connection: NodeControlConnectionState::Connected,
                release: converged_release(&desired.release),
                observed_capabilities: desired.required_capabilities.clone(),
                authorized_capabilities: desired.required_capabilities.clone(),
                observed_services: desired.required_services.clone(),
                evidence_refs: vec!["sha256:observation".to_string()],
                unknowns: Vec::new(),
            })
        }
    }

    struct Publisher;
    impl NodeStatusPublishPort for Publisher {
        async fn publish_node_status(&self, _plan: &NodeControlPlan) -> Result<String, String> {
            Ok("sha256:published-status".to_string())
        }
    }

    struct NoTransport;
    impl RuntimeArtifactTransportPort for NoTransport {
        async fn stage(
            &self,
            _source: &RuntimeArtifactSource,
            _destination: &Path,
            _expected_digest: Option<&str>,
        ) -> Result<
            crate::ports::runtime_artifact_transport_port::RuntimeArtifactTransferReceipt,
            String,
        > {
            panic!("converged and absent desired-state tests must not stage artifacts")
        }
    }

    struct NoActuation;
    impl RuntimeSeedActuationPort for NoActuation {
        fn apply(&self, _request: &RuntimeSeedRequest) -> RuntimeSeedApplyReport {
            panic!("converged and absent desired-state tests must not actuate a release")
        }

        fn persist(
            &self,
            _request: &RuntimeSeedRequest,
            _report: &RuntimeSeedApplyReport,
        ) -> Result<PathBuf, String> {
            panic!("converged and absent desired-state tests must not persist release evidence")
        }
    }

    struct ReconciliationObserver(Cell<u8>);
    impl NodeControlObservationPort for ReconciliationObserver {
        async fn observe_node_control(
            &self,
            desired: &NodeDesiredState,
        ) -> Result<NodeControlObservation, String> {
            let call = self.0.get();
            self.0.set(call + 1);
            let mut release = converged_release(&desired.release);
            if call == 0 {
                release.basket_present = false;
                release.detached_signature_present = false;
                release.basket_digest_read_back = None;
                release.signature = SecurityEvidenceState::Unknown;
                release.current_release_id = None;
                release.current_release_epoch = None;
                release.current_source_commit = None;
                release.current_build_proof_digest = None;
                release.current_basket_digest = None;
                release.postflight = SecurityEvidenceState::Unknown;
            }
            Ok(NodeControlObservation {
                schema: NODE_CONTROL_OBSERVATION_SCHEMA.to_string(),
                node_id: desired.node_id.clone(),
                observed_at_unix_ms: 1,
                observed_generation: Some(desired.generation),
                enrollment: SecurityEvidenceState::Proven,
                connection: NodeControlConnectionState::Connected,
                release,
                observed_capabilities: desired.required_capabilities.clone(),
                authorized_capabilities: desired.required_capabilities.clone(),
                observed_services: desired.required_services.clone(),
                evidence_refs: vec!["sha256:before".to_string()],
                unknowns: Vec::new(),
            })
        }
    }

    struct ProvedTransport;
    impl RuntimeArtifactTransportPort for ProvedTransport {
        async fn stage(
            &self,
            source: &RuntimeArtifactSource,
            destination: &Path,
            expected_digest: Option<&str>,
        ) -> Result<
            crate::ports::runtime_artifact_transport_port::RuntimeArtifactTransferReceipt,
            String,
        > {
            Ok(
                crate::ports::runtime_artifact_transport_port::RuntimeArtifactTransferReceipt {
                    schema: "cynic.runtime_artifact.transfer_receipt.v1",
                    status: "proved",
                    source: source.clone(),
                    destination: destination.display().to_string(),
                    expected_digest: expected_digest.map(str::to_string),
                    observed_digest: format!("sha256:{}", "a".repeat(64)),
                    bytes_written: 1,
                    mutates: true,
                    evidence_ref: format!("sha256:{}", "a".repeat(64)),
                    blockers: Vec::new(),
                },
            )
        }
    }

    struct ProvedActuation;
    impl RuntimeSeedActuationPort for ProvedActuation {
        fn apply(&self, request: &RuntimeSeedRequest) -> RuntimeSeedApplyReport {
            let after = converged_release(request);
            RuntimeSeedApplyReport {
                schema: "cynic.host.runtime_seed.apply_report.v3",
                status: "proved".to_string(),
                before: after.clone(),
                after: Some(after),
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
            Ok("/var/lib/cynic/runtime/evidence/release-2.json".into())
        }
    }

    fn desired() -> NodeDesiredState {
        NodeDesiredState {
            schema: NODE_DESIRED_STATE_SCHEMA.to_string(),
            node_id: "node-a".to_string(),
            role: "build_runner".to_string(),
            generation: 2,
            policy_digest: format!("sha256:{}", "a".repeat(64)),
            release: RuntimeSeedRequest {
                schema: RUNTIME_SEED_REQUEST_SCHEMA.to_string(),
                node_id: "node-a".to_string(),
                release_id: "release-2".to_string(),
                release_epoch: 2,
                source_commit: "c".repeat(40),
                build_proof_digest: format!("sha256:{}", "d".repeat(64)),
                basket_digest: format!("sha256:{}", "b".repeat(64)),
                registry_digest: format!("sha256:{}", "e".repeat(64)),
                basket_source: RuntimeArtifactSource {
                    transport: RuntimeArtifactTransportKind::Https,
                    locator: "https://artifacts.synapse.cynic/release-2.tar".to_string(),
                },
                basket_path: "/var/lib/cynic/inbox/release-2.tar".to_string(),
                detached_signature_source: RuntimeArtifactSource {
                    transport: RuntimeArtifactTransportKind::Https,
                    locator: "https://artifacts.synapse.cynic/release-2.tar.sig".to_string(),
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
                postflight_argv: vec![
                    "/usr/local/bin/cynic".to_string(),
                    "runtime".to_string(),
                    "paths".to_string(),
                    "--json".to_string(),
                ],
            },
            required_capabilities: vec!["build.execute".to_string()],
            required_services: vec!["cynic-node".to_string()],
            paused: false,
        }
    }

    fn converged_release(request: &RuntimeSeedRequest) -> RuntimeSeedObservation {
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
            previous_release_id: Some("release-1".to_string()),
            slot_layout: SecurityEvidenceState::Proven,
            postflight: SecurityEvidenceState::Proven,
            evidence_refs: vec!["sha256:release".to_string()],
            unknowns: Vec::new(),
        }
    }

    #[tokio::test]
    async fn tick_pulls_observes_reconciles_and_publishes_proof() {
        let report = reconcile_node_control_tick(
            "node-a",
            &Desired(Some(desired())),
            &Observer,
            &Publisher,
            &NoTransport,
            &NoActuation,
        )
        .await;
        assert_eq!(report.status, "converged");
        assert_eq!(
            report.status_evidence_ref.as_deref(),
            Some("sha256:published-status")
        );
    }

    #[tokio::test]
    async fn absent_desired_state_remains_explicitly_waiting() {
        let report = reconcile_node_control_tick(
            "node-a",
            &Desired(None),
            &Observer,
            &Publisher,
            &NoTransport,
            &NoActuation,
        )
        .await;
        assert_eq!(report.status, "waiting_for_desired_state");
        assert_eq!(report.unknowns, vec!["node_desired_state_unavailable"]);
    }

    #[tokio::test]
    async fn tick_transports_activates_reobserves_and_publishes_execution_proof() {
        let observer = ReconciliationObserver(Cell::new(0));

        let report = reconcile_node_control_tick(
            "node-a",
            &Desired(Some(desired())),
            &observer,
            &Publisher,
            &ProvedTransport,
            &ProvedActuation,
        )
        .await;

        assert_eq!(report.status, "converged");
        assert_eq!(observer.0.get(), 2);
        assert_eq!(
            report
                .plan
                .as_ref()
                .and_then(|plan| plan.execution.as_ref())
                .map(|execution| execution.status.as_str()),
            Some("proved")
        );
        assert_eq!(
            report
                .release_deployment
                .as_ref()
                .map(|deployment| deployment.transfers.len()),
            Some(2)
        );
    }
}
