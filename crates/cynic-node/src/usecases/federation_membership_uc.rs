//! Node-local federation membership tick (L3, D4).
//!
//! Each `cynic-node` owns its own membership view: it measures transport
//! liveness through the injected [`MembershipTransportPort`], derives the
//! [`LivenessObservation`] for its own heartbeat record, applies the pure
//! [`membership_liveness_transition`] law with the injectable
//! [`MembershipPolicy`], and persists the resulting typed view as a JSON
//! read-back under the runtime state root.
//!
//! Silence law: silence is measured from the **last acknowledged instant**
//! (the newest healthy evidence this node has ever recorded, threaded across
//! ticks through the persisted view) — never from the newest measurement.
//! Measuring from the newest measurement would keep a permanently-unreachable
//! transport forever fresh and freeze the member in `Active` forever.
//!
//! Authority: `protocols/machine-enrollment-federation.md` §3.2 (steady
//! state), decision D4 (node-local loop; the kernel only stores/projects).
//! The kernel is never contacted by this use case: when it dies, this tick
//! keeps working — that is what node-local means.

use std::time::{SystemTime, UNIX_EPOCH};

use cynic_network::TransportReadback;
use cynic_protocol::{
    LivenessObservation, MemberStatus, MembershipPolicy, MembershipPolicyState,
    membership_liveness_transition,
};

pub const MEMBERSHIP_VIEW_SCHEMA: &str = "cynic.federation_membership_view.v1";

/// Typed persisted view: one node's own membership state plus the evidence
/// that produced it. This is what a hub projection would later consume;
/// nothing here writes to any authority.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FederationMembershipView {
    pub schema: String,
    pub node_id: String,
    /// Policy validation blockers at tick time; empty list = policy usable.
    pub policy_blockers: MembershipPolicyState,
    pub status: MemberStatus,
    /// Latest per-plane transport evidence feeding the transition.
    pub transport_readbacks: Vec<TransportReadback>,
    /// Newest instant at which any plane proved healthy (drives silence).
    #[serde(default)]
    pub last_acknowledged_at_unix_ms: Option<u64>,
    /// Blockers that froze evaluation (invalid policy, bad transport report…).
    /// Frozen status + unknowns = explicit unknown, never fabricated health.
    pub unknowns: Vec<String>,
    pub observed_at_unix_ms: u64,
}

impl FederationMembershipView {
    /// Structural validity of the artifact itself. Policy blockers are DATA
    /// recorded on the view (see [`Self::policy_blockers`]), not structural
    /// defects: a view that honestly records an invalid policy is a valid
    /// view. Transport reports must still be structurally sound.
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != MEMBERSHIP_VIEW_SCHEMA {
            blockers.push("membership_view_schema_invalid".to_string());
        }
        if self.node_id.trim().is_empty() {
            blockers.push("membership_view_node_id_missing".to_string());
        }
        for rb in &self.transport_readbacks {
            blockers.extend(
                rb.validate()
                    .into_iter()
                    .map(|b| format!("transport[{}]: {}", rb.kind.plane_name(), b)),
            );
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }

    /// Newest healthy-evidence instant carried by this view's read-backs.
    pub fn freshest_acknowledgment(&self) -> Option<u64> {
        self.transport_readbacks
            .iter()
            .filter(|rb| rb.healthy())
            .map(|rb| rb.observed_at_unix_ms)
            .max()
            .or(self.last_acknowledged_at_unix_ms)
    }
}

/// Port over transport measurement. Implemented by adapters (the WireGuard
/// observer composes with an HTTP/TCP probe); tests inject fixtures.
pub trait MembershipTransportPort {
    fn observe_transport(&self) -> Vec<TransportReadback>;
}

/// Inputs of one tick, injected by the caller.
pub struct MembershipTickInput<'a> {
    pub node_id: &'a str,
    pub current_status: &'a MemberStatus,
    pub policy: &'a MembershipPolicy,
    pub transport: &'a dyn MembershipTransportPort,
    /// Newest previously-known healthy instant (threaded across ticks).
    pub last_acknowledged_at_unix_ms: Option<u64>,
    pub now_unix_ms: u64,
}

/// Output of one tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipTickOutcome {
    pub view: FederationMembershipView,
    /// True when the status changed compared to the input status.
    pub transitioned: bool,
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Derive one pure [`LivenessObservation`] from transport evidence plus the
/// threaded last-acknowledgment instant.
///
/// Law: acknowledged iff at least one read-back is healthy AND fresh against
/// twice the probe period (a stale healthy flag must not keep a dead member
/// alive). Otherwise silence is measured from the last acknowledged instant —
/// the newer of the injected history and any current healthy-but-stale
/// evidence — because measurements of unreachability do not reset silence.
pub fn liveness_from_readbacks(
    readbacks: &[TransportReadback],
    last_acknowledged_at_unix_ms: Option<u64>,
    now_unix_ms: u64,
    policy: &MembershipPolicy,
) -> LivenessObservation {
    let max_age = policy.probe_period_ms.saturating_mul(2);
    let mut freshest_current_ack: Option<u64> = None;
    let mut acknowledged = false;
    for rb in readbacks {
        let age = now_unix_ms.saturating_sub(rb.observed_at_unix_ms);
        if rb.healthy() && age <= max_age {
            acknowledged = true;
        }
        if rb.healthy() {
            freshest_current_ack = Some(
                freshest_current_ack.map_or(rb.observed_at_unix_ms, |best: u64| {
                    best.max(rb.observed_at_unix_ms)
                }),
            );
        }
    }
    if acknowledged {
        return LivenessObservation::Acknowledged;
    }
    // No live acknowledgment: measure silence from the last known one.
    let last_ack = match (last_acknowledged_at_unix_ms, freshest_current_ack) {
        (Some(history), Some(current)) => Some(history.max(current)),
        (opt, None) => opt,
        (None, Some(current)) => Some(current),
    };
    let elapsed = match last_ack {
        Some(ts) => now_unix_ms.saturating_sub(ts),
        None => u64::MAX, // never acknowledged: total silence
    };
    LivenessObservation::SilentFor {
        elapsed_ms: elapsed,
    }
}

/// Run one membership tick: observe → derive → transition → typed view.
/// Pure assembly over injected ports; I/O belongs to the caller (persistence).
pub fn run_membership_tick(input: MembershipTickInput<'_>) -> MembershipTickOutcome {
    let mut unknowns = Vec::new();
    let policy_blockers = input.policy.validate();
    if !policy_blockers.is_empty() {
        unknowns.extend(policy_blockers.iter().cloned());
    }

    let readbacks = input.transport.observe_transport();
    for rb in &readbacks {
        let transport_blockers = rb.validate();
        if !transport_blockers.is_empty() {
            unknowns.extend(
                transport_blockers
                    .into_iter()
                    .map(|b| format!("transport[{}]: {}", rb.kind.plane_name(), b)),
            );
        }
    }

    // Invalid policy freezes the transition (never guess); we still report.
    let (next_status, last_acknowledged_at_unix_ms) = if !policy_blockers.is_empty() {
        (
            input.current_status.clone(),
            input.last_acknowledged_at_unix_ms,
        )
    } else {
        let observation = liveness_from_readbacks(
            &readbacks,
            input.last_acknowledged_at_unix_ms,
            input.now_unix_ms,
            input.policy,
        );
        let next = membership_liveness_transition(input.current_status, &observation, input.policy);
        // On acknowledgment the ack instant advances to now.
        let ack = match observation {
            LivenessObservation::Acknowledged => Some(input.now_unix_ms),
            _ => {
                // carry forward history, enriched by current stale-healthy evidence
                let current_healthiest = readbacks
                    .iter()
                    .filter(|rb| rb.healthy())
                    .map(|rb| rb.observed_at_unix_ms)
                    .max();
                match (input.last_acknowledged_at_unix_ms, current_healthiest) {
                    (Some(h), Some(c)) => Some(h.max(c)),
                    (opt, None) => opt,
                    (None, Some(c)) => Some(c),
                }
            }
        };
        (next, ack)
    };

    let transitioned = &next_status != input.current_status;
    let view = FederationMembershipView {
        schema: MEMBERSHIP_VIEW_SCHEMA.to_string(),
        node_id: input.node_id.to_string(),
        policy_blockers,
        status: next_status,
        transport_readbacks: readbacks,
        last_acknowledged_at_unix_ms,
        unknowns,
        observed_at_unix_ms: input.now_unix_ms,
    };
    MembershipTickOutcome { view, transitioned }
}

// ── Persistence (thin, explicit path, pretty JSON read-back) ─────────────

fn view_path(state_root: &std::path::Path) -> std::path::PathBuf {
    state_root.join("federation").join("membership-view.json")
}

/// Persist a view under `<state_root>/federation/membership-view.json`.
/// Returns the written path. Sync fs on purpose: the volume is tiny and the
/// write is part of the tick's proof.
pub fn persist_membership_view(
    state_root: &std::path::Path,
    view: &FederationMembershipView,
) -> Result<std::path::PathBuf, String> {
    let dir = state_root.join("federation");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create federation dir: {e}"))?;
    let path = view_path(state_root);
    let body = serde_json::to_string_pretty(view).map_err(|e| format!("serialize view: {e}"))?;
    std::fs::write(&path, body + "\n").map_err(|e| format!("write view: {e}"))?;
    Ok(path)
}

/// Load a previously persisted view (read-back of our own proof artifact).
pub fn load_membership_view(
    state_root: &std::path::Path,
) -> Result<Option<FederationMembershipView>, String> {
    let path = view_path(state_root);
    if !path.exists() {
        return Ok(None);
    }
    let body = std::fs::read_to_string(&path).map_err(|e| format!("read view: {e}"))?;
    serde_json::from_str(&body)
        .map(Some)
        .map_err(|e| format!("parse view: {e}"))
}

/// One full tick against persisted state: load → decide → persist.
/// Returns the outcome. Used directly by tests and by the loop below.
pub fn tick_once(
    node_id: &str,
    policy: &MembershipPolicy,
    transport: &dyn MembershipTransportPort,
    state_root: &std::path::Path,
    now_unix_ms: u64,
) -> Result<MembershipTickOutcome, String> {
    let previous = load_membership_view(state_root)?;
    let (current_status, last_ack) = match &previous {
        Some(view) => (view.status.clone(), view.freshest_acknowledgment()),
        None => (MemberStatus::Active, None),
    };
    let outcome = run_membership_tick(MembershipTickInput {
        node_id,
        current_status: &current_status,
        policy,
        transport,
        last_acknowledged_at_unix_ms: last_ack,
        now_unix_ms,
    });
    persist_membership_view(state_root, &outcome.view)?;
    Ok(outcome)
}

/// Long-running loop wiring (tokio). Kept separate from the pure tick so the
/// decision layer stays unit-testable without a runtime.
pub async fn run_membership_loop(
    node_id: String,
    policy: MembershipPolicy,
    transport: std::sync::Arc<dyn MembershipTransportPort + Send + Sync>,
    state_root: std::path::PathBuf,
    interval_secs: u64,
    shutdown: tokio_util::sync::CancellationToken,
) {
    // First tick runs immediately: a starting node must produce its view
    // without waiting a full interval.
    let mut ticker =
        tokio::time::interval(std::time::Duration::from_secs(interval_secs.clamp(5, 300)));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        let now_ms = now_unix_ms();
        match tick_once(&node_id, &policy, transport.as_ref(), &state_root, now_ms) {
            Ok(outcome) => {
                if outcome.transitioned {
                    tracing::info!(
                        node_id = %node_id,
                        status = ?outcome.view.status,
                        "membership status transitioned"
                    );
                }
                if !outcome.view.unknowns.is_empty() {
                    tracing::warn!(
                        node_id = %node_id,
                        unknowns = ?outcome.view.unknowns,
                        "membership tick retained unknowns"
                    );
                }
            }
            Err(error) => {
                tracing::warn!(node_id = %node_id, %error, "membership tick failed");
            }
        }
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = ticker.tick() => continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_network::TransportReadback;
use cynic_protocol::{TransportKind, TransportState};
    use std::sync::Mutex;

    struct FixtureTransport(Vec<TransportReadback>);

    impl MembershipTransportPort for FixtureTransport {
        fn observe_transport(&self) -> Vec<TransportReadback> {
            self.0.clone()
        }
    }

    struct CountingTransport(Mutex<u32>);

    impl MembershipTransportPort for CountingTransport {
        fn observe_transport(&self) -> Vec<TransportReadback> {
            let mut guard = self.0.lock().expect("lock");
            *guard += 1;
            vec![]
        }
    }

    fn wg_readback(healthy: bool, at_ms: u64) -> TransportReadback {
        TransportReadback {
            schema: "cynic.transport_readback.v1".to_string(),
            node_id: "self".to_string(),
            kind: TransportKind::WireGuardPeer {
                endpoint: "10.10.10.119:44196".to_string(),
                allowed_ips: "10.20.0.5/32".to_string(),
            },
            state: if healthy {
                TransportState::Healthy
            } else {
                TransportState::Unreachable {
                    detail: "no_handshake_recorded".to_string(),
                }
            },
            observed_at_unix_ms: at_ms,
            evidence_refs: vec![],
        }
    }

    #[test]
    fn fresh_healthy_transport_is_acknowledged() {
        let policy = MembershipPolicy::default();
        let rbs = vec![wg_readback(true, 1000)];
        assert_eq!(
            liveness_from_readbacks(&rbs, None, 2000, &policy),
            LivenessObservation::Acknowledged
        );
    }

    #[test]
    fn stale_healthy_flag_does_not_keep_member_alive_but_counts_as_last_ack() {
        let policy = MembershipPolicy::default();
        // healthy evidence from t=0, now t=100k: beyond freshness window…
        let rbs = vec![wg_readback(true, 0)];
        assert_eq!(
            liveness_from_readbacks(&rbs, None, 100_000, &policy),
            LivenessObservation::SilentFor {
                elapsed_ms: 100_000
            }
        );
    }

    #[test]
    fn unreachable_measurements_do_not_reset_silence_clock() {
        // THE law this use case exists to enforce: fresh unreachable
        // measurements every probe period must NOT hide growing silence.
        let policy = MembershipPolicy::default();
        let rbs = vec![
            wg_readback(false, 90_000), // freshest measurement: 10s ago
            wg_readback(false, 50_000),
        ];
        let last_ack = Some(40_000u64); // last healthy handshake 60s ago
        assert_eq!(
            liveness_from_readbacks(&rbs, last_ack, 100_000, &policy),
            LivenessObservation::SilentFor { elapsed_ms: 60_000 }
        );
    }

    #[test]
    fn no_evidence_and_no_history_is_total_silence() {
        let policy = MembershipPolicy::default();
        assert_eq!(
            liveness_from_readbacks(&[], None, 1000, &policy),
            LivenessObservation::SilentFor {
                elapsed_ms: u64::MAX
            }
        );
    }

    #[test]
    fn silence_past_suspect_threshold_transitions_and_persists_reason() {
        let policy = MembershipPolicy::default(); // suspect at 40s
        let transport = FixtureTransport(vec![wg_readback(false, 55_000)]);
        let outcome = run_membership_tick(MembershipTickInput {
            node_id: "cynic-builder",
            current_status: &MemberStatus::Active,
            policy: &policy,
            transport: &transport,
            last_acknowledged_at_unix_ms: Some(55_000),
            now_unix_ms: 95_000, // 40s since last ack
        });
        assert!(outcome.transitioned);
        assert!(matches!(outcome.view.status, MemberStatus::Suspect { .. }));
        assert!(
            outcome.view.validate().is_empty(),
            "{:?}",
            outcome.view.validate()
        );
    }

    #[test]
    fn recovery_refutes_suspect_back_to_active_and_advances_ack() {
        let policy = MembershipPolicy::default();
        let transport = FixtureTransport(vec![wg_readback(true, 99_000)]);
        let outcome = run_membership_tick(MembershipTickInput {
            node_id: "cynic-builder",
            current_status: &MemberStatus::Suspect {
                reason: "silence".to_string(),
            },
            policy: &policy,
            transport: &transport,
            last_acknowledged_at_unix_ms: Some(55_000),
            now_unix_ms: 100_000,
        });
        assert!(outcome.transitioned);
        assert_eq!(outcome.view.status, MemberStatus::Active);
        assert_eq!(outcome.view.last_acknowledged_at_unix_ms, Some(100_000));
    }

    #[test]
    fn invalid_policy_freezes_view_and_records_unknowns() {
        let mut policy = MembershipPolicy::default();
        policy.lapsed_after_ms = 1000; // ordering violation vs suspect 40s
        let transport = FixtureTransport(vec![wg_readback(true, 1)]);
        let outcome = run_membership_tick(MembershipTickInput {
            node_id: "n",
            current_status: &MemberStatus::Active,
            policy: &policy,
            transport: &transport,
            last_acknowledged_at_unix_ms: None,
            now_unix_ms: 5000,
        });
        assert!(!outcome.transitioned);
        assert_eq!(outcome.view.status, MemberStatus::Active);
        assert!(
            outcome
                .view
                .unknowns
                .iter()
                .any(|u| u.contains("membership_policy")),
            "{:?}",
            outcome.view.unknowns
        );
        // The view is a VALID artifact even while recording an INVALID policy.
        assert!(
            outcome.view.validate().is_empty(),
            "{:?}",
            outcome.view.validate()
        );
    }

    #[test]
    fn view_roundtrips_through_json_file() {
        let tmp =
            std::env::temp_dir().join(format!("cynic-membership-test-{}", std::process::id()));
        let view = FederationMembershipView {
            schema: MEMBERSHIP_VIEW_SCHEMA.to_string(),
            node_id: "roundtrip".to_string(),
            policy_blockers: Vec::new(),
            status: MemberStatus::Lapsed {
                reason: "long_silence".to_string(),
            },
            transport_readbacks: vec![wg_readback(false, 1)],
            last_acknowledged_at_unix_ms: Some(7),
            unknowns: vec![],
            observed_at_unix_ms: 42,
        };
        let path = persist_membership_view(&tmp, &view).expect("persist");
        assert!(path.ends_with("membership-view.json"));
        let loaded = load_membership_view(&tmp).expect("load").expect("present");
        assert_eq!(loaded, view);
        std::fs::remove_dir_all(&tmp).expect("cleanup");
    }

    #[test]
    fn tick_once_threads_last_ack_across_persisted_views() {
        let tmp =
            std::env::temp_dir().join(format!("cynic-membership-thread-{}", std::process::id()));
        let policy = MembershipPolicy::default();
        let silent = FixtureTransport(vec![wg_readback(false, 1000)]);

        // t=2k: acknowledged (fresh healthy at t=1k)
        let healthy = FixtureTransport(vec![wg_readback(true, 1000)]);
        let o1 = tick_once("n", &policy, &healthy, &tmp, 2_000).expect("tick1");
        assert_eq!(o1.view.status, MemberStatus::Active);

        // t=50k: silence since ack(t=2k) = 48s ≥ 40s → Suspect
        let o2 = tick_once("n", &policy, &silent, &tmp, 50_000).expect("tick2");
        assert!(matches!(o2.view.status, MemberStatus::Suspect { .. }));

        // t=190k: fresh unreachable measurements keep arriving, but last ack is
        // still t=2k → silence 188s ≥ 180s → Lapsed. Anti-freeze law:
        // unreachable measurements never reset the silence clock.
        let o3 = tick_once("n", &policy, &silent, &tmp, 190_000).expect("tick3");
        assert!(matches!(o3.view.status, MemberStatus::Lapsed { .. }));

        std::fs::remove_dir_all(&tmp).expect("cleanup");
    }

    #[test]
    fn loop_consumes_transport_each_tick_until_shutdown() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("rt");
        rt.block_on(async move {
            let counting = std::sync::Arc::new(CountingTransport(Mutex::new(0)));
            let shutdown = tokio_util::sync::CancellationToken::new();
            let handle_token = shutdown.clone();
            let counting_for_task = counting.clone();
            let tmp =
                std::env::temp_dir().join(format!("cynic-membership-loop-{}", std::process::id()));
            let tmp_for_cleanup = tmp.clone();
            let task = tokio::spawn(async move {
                run_membership_loop(
                    "loop-node".to_string(),
                    MembershipPolicy::default(),
                    counting_for_task,
                    tmp,
                    5,
                    handle_token,
                )
                .await;
            });
            // first tick runs immediately; give it a moment, then shut down
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            shutdown.cancel();
            task.await.expect("task joins");
            let ticks = *counting.0.lock().expect("lock");
            assert!(ticks >= 1, "loop should have observed at least once");
            std::fs::remove_dir_all(&tmp_for_cleanup).ok();
        });
    }
}
