//! Explicit node-owned consumer for resident takeover activation requests.
//!
//! Activation is opt-in through `CYNIC_TAKEOVER_ACTIVATION_APPLY=1`. The
//! default path only observes the spool and performs no host mutation.

use std::time::Duration;

use cynic_host::runtime_takeover_activation::{
    RuntimeTakeoverActivationReport, apply_runtime_takeover_activation,
    plan_runtime_takeover_activation,
};
use cynic_lifecycle::LocalArgvRunner;
use cynic_protocol::decide_runtime_takeover;
use tokio_util::sync::CancellationToken;

mod store;
use store::{JsonlTakeoverWatchStore, TakeoverWatchStore};

pub fn maybe_apply_from_watch(
    node_control_authorized: bool,
    node_id: &str,
) -> Result<Option<RuntimeTakeoverActivationReport>, String> {
    let apply_enabled = activation_apply_enabled(
        std::env::var("CYNIC_TAKEOVER_ACTIVATION_APPLY")
            .ok()
            .as_deref(),
    );
    if !apply_enabled {
        return Ok(None);
    }
    if !takeover_mutation_authorized(apply_enabled, node_control_authorized) {
        return Err("takeover activation requires proven node_control capability".to_string());
    }
    let store = JsonlTakeoverWatchStore::from_env()?;
    maybe_apply_from_store(&store, node_id)
}

fn maybe_apply_from_store(
    store: &dyn TakeoverWatchStore,
    node_id: &str,
) -> Result<Option<RuntimeTakeoverActivationReport>, String> {
    let record = store
        .latest()?
        .ok_or_else(|| "takeover activation watch has no records".to_string())?;
    let request = record
        .activation_request
        .ok_or_else(|| "latest takeover record has no activation request".to_string())?;
    if !takeover_request_targets_node(&request.node_id, node_id) {
        return Err(
            "takeover activation target node does not match local node identity".to_string(),
        );
    }
    if !authenticated_kernel_admission_ref(&request.admission_proof_ref) {
        return Err(
            "takeover activation admission proof is not kernel-authenticated; spool references are evidence only"
                .to_string(),
        );
    }
    let decision = decide_runtime_takeover(record.observation.clone());
    if record.schema != cynic_node_protocol::RUNTIME_TAKEOVER_WATCH_SCHEMA
        || !record.decision.promote
        || !decision.promote
    {
        return Err("takeover activation admission is blocked".to_string());
    }
    if store.acknowledged(&request.admission_proof_ref)? {
        return Ok(None);
    }
    let plan = plan_runtime_takeover_activation(request.clone(), &decision)?;
    let report = apply_runtime_takeover_activation(&plan, &LocalArgvRunner);
    let ack = cynic_node_protocol::RuntimeTakeoverActivationAck {
        schema: cynic_node_protocol::RUNTIME_TAKEOVER_ACTIVATION_ACK_SCHEMA.to_string(),
        admission_proof_ref: request.admission_proof_ref,
        node_id: request.node_id,
        fencing_token: request.fencing_token,
        status: report.status.to_string(),
        evidence_refs: report
            .observation
            .as_ref()
            .map(|observation| observation.proof_refs.clone())
            .unwrap_or_default(),
        blockers: report.blockers.clone(),
    };
    store.acknowledge(&ack)?;
    if report.status != "activated_proved" {
        return Err(format!(
            "takeover activation read-back blocked: {:?}",
            report.blockers
        ));
    }
    Ok(Some(report))
}

/// Poll the durable admission spool without requiring a node restart. The
/// apply gate is checked on every pass so changing it cannot retroactively
/// authorize a mutation unless the explicit value is present.
pub fn spawn_activation_loop<F>(
    shutdown: CancellationToken,
    node_id: String,
    node_control_authorized: F,
) -> tokio::task::JoinHandle<()>
where
    F: Fn() -> bool + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = interval.tick() => {
                    let authorized = node_control_authorized();
                    let target_node_id = node_id.clone();
                    match tokio::task::spawn_blocking(move || maybe_apply_from_watch(authorized, &target_node_id)).await {
                        Ok(Ok(Some(report))) => tracing::info!(status = report.status, "takeover activation read-back completed"),
                        Ok(Ok(None)) => {}
                        Ok(Err(error)) => tracing::warn!(error = %error, "takeover activation remained blocked"),
                        Err(error) => tracing::warn!(error = %error, "takeover activation worker failed"),
                    }
                }
            }
        }
    })
}

fn activation_apply_enabled(value: Option<&str>) -> bool {
    value == Some("1")
}

fn takeover_mutation_authorized(apply_enabled: bool, node_control_authorized: bool) -> bool {
    apply_enabled && node_control_authorized
}

fn takeover_request_targets_node(request_node_id: &str, local_node_id: &str) -> bool {
    !local_node_id.trim().is_empty() && request_node_id == local_node_id
}

/// A reference string is never proof by itself. No kernel verifier exists yet,
/// so the activation boundary remains fail-closed until one is injected.
fn authenticated_kernel_admission_ref(_admission_proof_ref: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn missing_apply_gate_is_read_only() {
        assert!(!activation_apply_enabled(None));
        assert!(!activation_apply_enabled(Some("0")));
        assert!(activation_apply_enabled(Some("1")));
    }

    #[test]
    fn apply_gate_never_substitutes_for_node_authority() {
        assert!(!takeover_mutation_authorized(true, false));
        assert!(!takeover_mutation_authorized(false, true));
        assert!(takeover_mutation_authorized(true, true));
    }

    #[test]
    fn activation_request_is_bound_to_the_local_node() {
        assert!(takeover_request_targets_node("node-a", "node-a"));
        assert!(!takeover_request_targets_node("node-a", "node-b"));
        assert!(!takeover_request_targets_node("node-a", ""));
    }

    #[test]
    fn spool_location_is_not_treated_as_an_admission_proof() {
        assert!(!authenticated_kernel_admission_ref(
            "spool://takeover-watch/4"
        ));
        assert!(!authenticated_kernel_admission_ref(
            "kernel://takeover-admission/4"
        ));
    }

    #[test]
    fn blocked_ack_does_not_suppress_retry() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join("ack.json");
        fs::write(
            &path,
            r#"{"schema":"cynic.runtime_takeover.activation_ack.v1","admission_proof_ref":"spool://proof","node_id":"node-a","fencing_token":4,"status":"blocked","evidence_refs":[],"blockers":["fixture"]}"#,
        )
        .expect("ack");
        assert!(!store::ack_matches(&path, "spool://proof").expect("ack read"));
        fs::write(
            &path,
            r#"{"schema":"cynic.runtime_takeover.activation_ack.v1","admission_proof_ref":"spool://proof","node_id":"node-a","fencing_token":4,"status":"activated_proved","evidence_refs":["fixture"],"blockers":[]}"#,
        )
        .expect("ack");
        assert!(store::ack_matches(&path, "spool://proof").expect("ack read"));
    }
}
