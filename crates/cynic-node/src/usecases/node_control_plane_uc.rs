use cynic_protocol::{NodeControlPlan, reconcile_node_control};
use serde::Serialize;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::ports::node_control_plane_port::{
    NodeControlObservationPort, NodeDesiredStatePullPort, NodeStatusPublishPort,
};

pub const NODE_CONTROL_TICK_REPORT_SCHEMA: &str = "cynic.node_control_tick_report.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeControlTickReport {
    pub schema: &'static str,
    pub status: String,
    pub node_id: String,
    pub desired_generation: Option<u64>,
    pub plan: Option<NodeControlPlan>,
    pub status_evidence_ref: Option<String>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
}

pub async fn reconcile_node_control_tick<D, O, S>(
    node_id: &str,
    desired_state: &D,
    observer: &O,
    status_publisher: &S,
) -> NodeControlTickReport
where
    D: NodeDesiredStatePullPort,
    O: NodeControlObservationPort,
    S: NodeStatusPublishPort,
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
                Vec::new(),
                vec![format!("node_control_observation_failed:{error}")],
            );
        }
    };
    let plan = reconcile_node_control(&desired, &observation);
    let desired_generation = Some(plan.desired_generation);
    let plan_status = plan.status.clone();
    match status_publisher.publish_node_status(&plan).await {
        Ok(evidence_ref) if !evidence_ref.trim().is_empty() => report(
            node_id,
            &plan_status,
            desired_generation,
            Some(plan),
            Some(evidence_ref),
            Vec::new(),
            Vec::new(),
        ),
        Ok(_) => report(
            node_id,
            "unknown",
            desired_generation,
            Some(plan),
            None,
            Vec::new(),
            vec!["node_status_publish_proof_missing".to_string()],
        ),
        Err(error) => report(
            node_id,
            "degraded_disconnected",
            desired_generation,
            Some(plan),
            None,
            vec![format!("node_status_publish_failed:{error}")],
            Vec::new(),
        ),
    }
}

pub async fn run_node_control_loop<D, O, S>(
    node_id: &str,
    interval: Duration,
    desired_state: &D,
    observer: &O,
    status_publisher: &S,
    shutdown: &CancellationToken,
) where
    D: NodeDesiredStatePullPort,
    O: NodeControlObservationPort,
    S: NodeStatusPublishPort,
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
        status_evidence_ref,
        blockers,
        unknowns,
    }
}

#[cfg(test)]
mod tests {
    use cynic_protocol::{
        NODE_CONTROL_OBSERVATION_SCHEMA, NODE_DESIRED_STATE_SCHEMA, NodeControlConnectionState,
        NodeControlObservation, NodeDesiredState, RUNTIME_SEED_OBSERVATION_SCHEMA,
        RUNTIME_SEED_REQUEST_SCHEMA, RuntimeSeedObservation, RuntimeSeedRequest,
        SecurityEvidenceState,
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
                basket_digest: format!("sha256:{}", "b".repeat(64)),
                basket_path: "/var/lib/cynic/inbox/release-2.tar".to_string(),
                detached_signature_path: "/var/lib/cynic/inbox/release-2.tar.sig".to_string(),
                allowed_signers_path: "/etc/cynic/runtime-allowed-signers".to_string(),
                signer_identity: "release-authority".to_string(),
                signature_namespace: "cynic-runtime-seed".to_string(),
                required_seed_contract_revision: 1,
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
            seed_contract_revision: Some(1),
            trust_root: SecurityEvidenceState::Proven,
            signature: SecurityEvidenceState::Proven,
            basket_present: true,
            basket_digest_read_back: Some(request.basket_digest.clone()),
            current_release_id: Some(request.release_id.clone()),
            current_release_epoch: Some(request.release_epoch),
            current_basket_digest: Some(request.basket_digest.clone()),
            previous_release_id: Some("release-1".to_string()),
            slot_layout: SecurityEvidenceState::Proven,
            postflight: SecurityEvidenceState::Proven,
            evidence_refs: vec!["sha256:release".to_string()],
            unknowns: Vec::new(),
        }
    }

    #[tokio::test]
    async fn tick_pulls_observes_reconciles_and_publishes_proof() {
        let report =
            reconcile_node_control_tick("node-a", &Desired(Some(desired())), &Observer, &Publisher)
                .await;
        assert_eq!(report.status, "converged");
        assert_eq!(
            report.status_evidence_ref.as_deref(),
            Some("sha256:published-status")
        );
    }

    #[tokio::test]
    async fn absent_desired_state_remains_explicitly_waiting() {
        let report =
            reconcile_node_control_tick("node-a", &Desired(None), &Observer, &Publisher).await;
        assert_eq!(report.status, "waiting_for_desired_state");
        assert_eq!(report.unknowns, vec!["node_desired_state_unavailable"]);
    }
}
