use std::sync::Arc;
use std::time::Duration;

use crate::domain::coord::CoordPort;
use crate::domain::storage::StoragePort;
use crate::infra::local_spool::LocalEvidenceSpool;
use crate::infra::runtime_takeover::{RuntimeTakeoverController, configured_standby_mode};
use tokio_util::sync::CancellationToken;

/// Observe resident takeover readiness and persist only the admission proof.
/// The task deliberately has no activation adapter: a promotable observation
/// is evidence for an explicitly authorized supervisor, never a hidden mode
/// switch inside the kernel process.
pub fn spawn_resident_takeover_watch(
    coord: Arc<dyn CoordPort>,
    storage: Arc<dyn StoragePort>,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if configured_standby_mode() != cynic_protocol::StandbyMode::Resident {
            return;
        }
        let controller = RuntimeTakeoverController {};
        let spool = LocalEvidenceSpool::from_env("takeover-watch");
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = interval.tick() => {
                    let storage_available = tokio::time::timeout(
                        Duration::from_secs(5),
                        storage.ping(),
                    ).await.is_ok_and(|result| result.is_ok());
                    let candidate_ready = spool.as_ref().is_some_and(LocalEvidenceSpool::ready);
                    let observation = controller
                        .observe(&coord, candidate_ready, storage_available)
                        .await;
                    let decision = controller.decide(observation.clone());
                    if decision.promote {
                        tracing::warn!(
                            "resident takeover admission proved; activation remains external and explicit"
                        );
                    }
                    if let Some(spool) = &spool {
                        if spool.ready() {
                            let observed_at_unix_ms = now_unix_ms();
                            let admission_proof = if decision.promote {
                                let proof = cynic_protocol::RuntimeTakeoverAdmissionProof {
                                    schema: cynic_protocol::RUNTIME_TAKEOVER_ADMISSION_PROOF_SCHEMA.to_string(),
                                    proof_ref: format!("kernel://takeover-admission/{observed_at_unix_ms}"),
                                    node_id: std::env::var("CYNIC_NODE_ID").unwrap_or_default(),
                                    observed_at_unix_ms,
                                    observation: observation.clone(),
                                    decision: cynic_protocol::RuntimeTakeoverWatchDecision {
                                        status: decision.status.to_string(),
                                        promote: decision.promote,
                                        fencing_token: decision.fencing_token,
                                    },
                                };
                                match coord.record_takeover_admission(&proof).await {
                                    Ok(read_back) if read_back == proof => Some(read_back),
                                    Ok(_) => {
                                        tracing::warn!("takeover admission durable read-back mismatched");
                                        None
                                    }
                                    Err(error) => {
                                        tracing::warn!(error = %error, "takeover admission persistence unavailable; supervisor activation remains blocked");
                                        None
                                    }
                                }
                            } else {
                                None
                            };
                            let activation_request = admission_proof.map(|proof| {
                                cynic_protocol::RuntimeTakeoverActivationRequest {
                                    schema: cynic_protocol::RUNTIME_TAKEOVER_ACTIVATION_SCHEMA
                                        .to_string(),
                                    node_id: std::env::var("CYNIC_NODE_ID").unwrap_or_default(),
                                    fencing_token: decision.fencing_token,
                                    prior_fencing_token: observation.prior_fencing_token,
                                    admission_proof_ref: proof.proof_ref,
                                    standby_unit: std::env::var("CYNIC_STANDBY_UNIT")
                                        .unwrap_or_else(|_| {
                                            "cynic-kernel-standby.service".to_string()
                                        }),
                                    active_unit: std::env::var("CYNIC_ACTIVE_UNIT")
                                        .unwrap_or_else(|_| "cynic-kernel.service".to_string()),
                                    health_endpoint: std::env::var("CYNIC_ACTIVE_HEALTH_ENDPOINT")
                                        .unwrap_or_else(|_| {
                                            "http://127.0.0.1:3030/health".to_string()
                                        }),
                                    identity_endpoint: std::env::var(
                                        "CYNIC_ACTIVE_IDENTITY_ENDPOINT",
                                    )
                                    .unwrap_or_else(|_| {
                                        "http://127.0.0.1:3030/identity".to_string()
                                    }),
                                }
                            });
                            let record = cynic_protocol::RuntimeTakeoverWatchRecord {
                                schema: cynic_protocol::RUNTIME_TAKEOVER_WATCH_SCHEMA.to_string(),
                                observed_at_unix_ms,
                                observation,
                                decision: cynic_protocol::RuntimeTakeoverWatchDecision {
                                    status: decision.status.to_string(),
                                    promote: decision.promote,
                                    fencing_token: decision.fencing_token,
                                },
                                activation_request,
                            };
                            if let Err(error) = spool.append(&record) {
                                tracing::warn!(error = %error, "takeover watch proof append failed");
                            }
                        }
                    }
                }
            }
        }
    })
}

fn now_unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Renew the primary takeover lease only for an explicitly active node.
/// Resident/unknown modes never mutate coordination state.
pub fn spawn_primary_takeover_lease(
    coord: Arc<dyn CoordPort>,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    let node_id = std::env::var("CYNIC_NODE_ID")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let active = configured_standby_mode() == cynic_protocol::StandbyMode::Active;
    tokio::spawn(async move {
        let Some(node_id) = node_id else {
            klog!("[takeover] primary lease disabled: CYNIC_NODE_ID is absent");
            return;
        };
        if !active {
            klog!("[takeover] primary lease disabled: node is not active");
            return;
        }
        let controller = RuntimeTakeoverController {};
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[takeover] primary lease loop stopped");
                    break;
                }
                _ = interval.tick() => {
                    match tokio::time::timeout(
                        Duration::from_secs(10),
                        controller.publish_primary_lease(&coord, &node_id, 90),
                    ).await {
                        Ok(Ok(observation)) => {
                            if cynic_protocol::takeover_lease_owned_by(&observation, &node_id) {
                                tracing::debug!(
                                    node_id = %node_id,
                                    fencing_token = observation.fencing_token,
                                    "primary takeover lease renewed"
                                );
                            } else {
                                klog!("[takeover] primary lease read-back lost; stopping active runtime");
                                shutdown.cancel();
                                break;
                            }
                        }
                        Ok(Err(error)) => {
                            let fenced = error.contains("fenced by active primary");
                            tracing::warn!(error = %error, fenced, "primary takeover lease renewal failed");
                            if fenced {
                                shutdown.cancel();
                                break;
                            }
                        }
                        Err(_) => tracing::warn!("primary takeover lease renewal timed out"),
                    }
                }
            }
        }
    })
}
