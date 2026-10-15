mod evidence;
mod execute;
mod lifecycle;
mod measure;
mod prepare;
mod release_publication;
mod source_prepare;

use std::time::Duration;

use cynic_build::{QueuedCandidate, canonical_build_action_from_argv, plan_build_queue};
use cynic_host::adapters::local_capacity::LocalCapacityAdapter;
use cynic_protocol::build_latency::{
    SCCACHE_ADOPT_THRESHOLD, RSS_CAP_MB,
    decide_admit_parallel_slot, decide_reuse_stable_checkout, decide_require_sccache,
};
use cynic_protocol::capacity::{
    CapacityAssessmentV1, LoadPolicy, LoadTier, assess_load_tier,
};
use cynic_workload::{AgentWorkloadState, DurableAgentWorkloadRecord};
use tokio_util::sync::CancellationToken;

use crate::{
    adapters::local_agent_workload_store::LocalAgentWorkloadStore,
    config::{BuildSchedulerConfig, NodeControlConfig},
    ports::agent_workload_store_port::AgentWorkloadStorePort,
};

pub async fn run_loop(control: NodeControlConfig, shutdown: CancellationToken) {
    let Some(scheduler) = control.build_scheduler.clone() else {
        return;
    };
    let store = LocalAgentWorkloadStore::new(
        control
            .agent_workload_state_root
            .join("agent-scheduler/workloads"),
    );
    let interval = Duration::from_secs(scheduler.interval_secs);
    loop {
        if let Err(error) = run_iteration(&control, &scheduler, &store, &shutdown).await {
            tracing::warn!(%error, "build scheduler iteration blocked");
        }
        tokio::select! {
            () = shutdown.cancelled() => break,
            () = tokio::time::sleep(interval) => {}
        }
    }
}

/// Order this node's build records FIFO per the queue policy. Pure mapping
/// over durable records; the loop below only follows the returned order.
pub fn queue_ordered_ids(records: &[DurableAgentWorkloadRecord], node_id: &str) -> Vec<String> {
    let candidates = records
        .iter()
        .filter(|durable| durable.record.envelope.delegation.node_id == node_id)
        .map(|durable| QueuedCandidate {
            workload_id: durable.record.envelope.workload_id.clone(),
            idempotency_key: durable.record.envelope.idempotency_key.clone(),
            committed_at_ms: durable.committed_at_ms,
            source_commit: durable.record.envelope.source_commit.clone(),
            branch: durable.record.envelope.branch.clone(),
            terminal: durable.record.state.terminal(),
            is_build: canonical_build_action_from_argv(&durable.record.envelope.action.argv)
                .is_some(),
        })
        .collect();
    plan_build_queue(candidates).ordered
}

/// Pure capacity gate for admission: admit a claim unless the assessment
/// is `Saturated`. Unknown measurements (Normal + residual unknowns)
/// proceed — the policy default never refuses on unmeasured capacity.
fn admit_claim_under_capacity(assessment: &CapacityAssessmentV1) -> bool {
    assessment.state != LoadTier::Saturated
}

/// Conservative per-job RSS estimate in MB, grounded in measurement.
/// Source: infra/build-policy.json → parallelism.measured_alternative.peak_rss_mib
///   - single_job = 1166 MiB (measured on cynic-builder CT105, rustc 1.95.0)
///   - two_jobs   = 1255 MiB (Δ +7.6% — second job shares target-dir pages)
/// 1200 MB is a conservative per-job projection: it rounds up the measured
/// single-job peak (1166) and stays below the two-job peak (1255), so any
/// single-job build is admitted unless the cap is set below 1200.
const RSS_PER_JOB_MB: u64 = 1200;

/// Project the peak RSS for a build based on the configured job count.
/// `build_jobs = None` means cargo's default (1 job). Pure.
pub(crate) fn projected_rss_mb(build_jobs: Option<usize>) -> u64 {
    let jobs = build_jobs.unwrap_or(1);
    jobs as u64 * RSS_PER_JOB_MB
}
// NOTE: no local cap-gate helper here by design. Admission uses the measured
// T6 predicate `decide_admit_parallel_slot` (cynic-protocol, H2); the call
// site resolves `None` to `RSS_CAP_MB` before calling it. A second helper
// would be a second authority over one decision.

/// Admission decision with T6 predicate consultation results.
///
/// Each field mirrors one frozen predicate from `cynic_protocol::build_latency`:
/// - `stable_checkout_reuse` ← `decide_reuse_stable_checkout`
/// - `parallel_slot_admitted` ← `decide_admit_parallel_slot`
/// - `sccache_required` ← `decide_require_sccache`
///
/// `admitted` is the composite gate: capacity + RSS cap must both admit.
/// The predicate fields inform behavior and are logged on refusal.
pub(crate) struct AdmissionDecision {
    pub admitted: bool,
    pub stable_checkout_reuse: bool,
    pub parallel_slot_admitted: bool,
    pub sccache_required: bool,
}

/// Combined admission decision for a scheduler tick. Consults all three
/// T6 predicates at the decision point so behavior changes only where a
/// predicate says so. The load-tier gate and RSS-cap gate must both admit;
/// the stable-checkout and sccache predicates inform behavior and are
/// logged on refusal.
///
/// Predicate wiring:
/// - H1 `decide_reuse_stable_checkout`: stable root configured ⇒ reuse.
/// - H2 `decide_admit_parallel_slot`: projected RSS ≤ cap ⇒ admit.
///   `RSS_CAP_MB` is the default cap; `max_rss_cap_mb` overrides it.
/// - H3 `decide_require_sccache`: measured hit rate ≥ floor ⇒ required.
///   Consulted only when `sccache_measured_hit_rate` is `Some`.
pub(crate) fn should_admit_build(
    assessment: &CapacityAssessmentV1,
    scheduler: &BuildSchedulerConfig,
) -> AdmissionDecision {
    let refused = |parallel_slot_admitted| AdmissionDecision {
        admitted: false,
        stable_checkout_reuse: false,
        parallel_slot_admitted,
        sccache_required: false,
    };
    if !admit_claim_under_capacity(assessment) {
        return refused(false);
    }
    let projected = projected_rss_mb(scheduler.build_jobs);
    let rss_cap = scheduler.max_rss_cap_mb.unwrap_or(RSS_CAP_MB);
    let parallel_slot_admitted = decide_admit_parallel_slot(rss_cap, projected);
    if !parallel_slot_admitted {
        return refused(false);
    }
    let stable_root_configured = scheduler.stable_workspace_root.is_some();
    let stable_checkout_reuse = decide_reuse_stable_checkout(stable_root_configured);
    let sccache_required = scheduler
        .sccache_measured_hit_rate
        .map(|measured| decide_require_sccache(SCCACHE_ADOPT_THRESHOLD, measured))
        .unwrap_or(false);
    AdmissionDecision {
        admitted: true,
        stable_checkout_reuse,
        parallel_slot_admitted,
        sccache_required,
    }
}

/// Canonical tier name for proofs and logs. Mirrors the serde snake_case
/// representation of `LoadTier` without serialising the whole assessment.
fn load_tier_str(tier: &LoadTier) -> &'static str {
    match tier {
        LoadTier::Headroom => "headroom",
        LoadTier::Normal => "normal",
        LoadTier::Saturated => "saturated",
    }
}

/// One capacity assessment per scheduler tick. Local observation via the
/// existing `cynic-host` adapter (already a `cynic-node` dependency);
/// on observation failure the unmeasured path yields Normal + explicit
/// unknowns, and the policy default proceeds per `assess_load_tier`.
fn measure_capacity_assessment() -> CapacityAssessmentV1 {
    let policy = LoadPolicy::with_legacy_defaults("build-scheduler");
    match LocalCapacityAdapter::new().assess_capacity(&policy) {
        Ok(assessment) => assessment,
        Err(_) => assess_load_tier(None, None, &policy),
    }
}

async fn run_iteration(
    control: &NodeControlConfig,
    scheduler: &BuildSchedulerConfig,
    store: &LocalAgentWorkloadStore,
    shutdown: &CancellationToken,
) -> Result<(), String> {
    // Capacity gate at admission (T4): measure once per tick. A Saturated host
    // skips claiming entirely — records stay queued; refusal is per-iteration
    // so a later tick may claim once headroom returns.
    let capacity = measure_capacity_assessment();
    let load_tier = load_tier_str(&capacity.state).to_string();
    let capacity_unknowns = capacity.residual_unknowns.clone();
    let decision = should_admit_build(&capacity, scheduler);
    if !decision.admitted {
        tracing::info!(
            tier = %load_tier,
            stable_checkout_reuse = %decision.stable_checkout_reuse,
            parallel_slot_admitted = %decision.parallel_slot_admitted,
            sccache_required = %decision.sccache_required,
            "build scheduler tick skipped: admission predicate refused"
        );
        return Ok(());
    }
    let durables = store.durable_records().map_err(|error| error.code)?;
    for workload_id in queue_ordered_ids(&durables, &control.node_id) {
        if shutdown.is_cancelled() {
            return Ok(());
        }
        let Some(current) = store.read(&workload_id).map_err(|error| error.code)? else {
            continue;
        };
        if current.record.state == AgentWorkloadState::CancelRequested {
            lifecycle::acknowledge_cancelled(
                store,
                &current,
                "cancellation_observed_before_execution",
            )?;
            measure::persist_cancellation_proof(&current.record.envelope.workload_id, current.revision);
            continue;
        }
        let recovery_candidate = current.record.claim.as_ref().is_some_and(|claim| {
            current.record.state == AgentWorkloadState::Claimed
                && claim.expires_at_ms <= lifecycle::current_unix_ms()
        });
        let Some(claimed) = lifecycle::claim_if_available(control, scheduler, store, &current)?
        else {
            continue;
        };
        if recovery_candidate {
            measure::persist_recovery_proof(
                &claimed.record.envelope.workload_id,
                current.revision,
                claimed.revision,
            );
        }
        execute::execute_claimed(
            control,
            scheduler,
            store,
            claimed,
            shutdown,
            &load_tier,
            &capacity_unknowns,
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::agent_workload_store_port::AgentWorkloadStorePort;
    use crate::usecases::build_admission::admit_build_check;
    use cynic_protocol::AgentCompletionStatus;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static QUEUE_SEQ: AtomicU64 = AtomicU64::new(0);

    const QA: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4";
    const QB: &str = "b1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4";

    fn queue_store() -> LocalAgentWorkloadStore {
        let id = QUEUE_SEQ.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("cynic-queue-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create queue store dir");
        LocalAgentWorkloadStore::new(&dir)
    }

    #[test]
    fn queue_follows_commit_time_not_insertion_order() {
        let store = queue_store();
        admit_build_check(
            QB,
            "cynic-host",
            "cynic-builder",
            "node-1",
            "agent-1",
            200,
            &store,
        )
        .expect("admit newer first");
        admit_build_check(
            QA,
            "cynic-host",
            "cynic-builder",
            "node-1",
            "agent-1",
            100,
            &store,
        )
        .expect("admit older second");

        let durables = store.durable_records().expect("list records");
        let ordered = queue_ordered_ids(&durables, "node-1");

        assert_eq!(ordered.len(), 2);
        assert!(ordered[0].contains(&QA[..12]));
        assert!(ordered[1].contains(&QB[..12]));
    }

    #[test]
    fn queue_skips_terminal_and_foreign_records() {
        let store = queue_store();
        let done = admit_build_check(
            QA,
            "cynic-host",
            "cynic-builder",
            "node-1",
            "agent-1",
            100,
            &store,
        )
        .expect("admit");
        admit_build_check(
            QB,
            "cynic-host",
            "cynic-builder",
            "node-9",
            "agent-1",
            50,
            &store,
        )
        .expect("admit foreign");
        let mut finished = done.record.clone();
        finished.state = AgentWorkloadState::Completed;
        store
            .compare_and_swap(
                &done.record.envelope.workload_id,
                Some(done.revision),
                &finished,
                150,
            )
            .expect("complete first");

        let durables = store.durable_records().expect("list records");
        let ordered = queue_ordered_ids(&durables, "node-1");

        assert!(ordered.is_empty());
    }

    /// Regression: settlement proof persists reasons, timings, and stage
    /// durations. The settlement evidence ref must be a
    /// `scheduler-settlement:{digest}` over the canonical JSON including
    /// `prepare_ms`, `execute_ms`, `queue_wait_ms`, and `queue_wait_clamped`.
    #[test]
    fn settlement_proof_persists_reasons_and_timings() {
        let status = AgentCompletionStatus::Blocked;
        let unknowns = vec![
            "build_execution_timeout".to_string(),
            "build_peak_memory_unobserved".to_string(),
        ];
        let fields = measure::SettlementProofFields {
            workload_id: "test-workload-123",
            status: &status,
            unknowns: &unknowns,
            elapsed: Duration::from_millis(1500),
            measured_cpu_ms: Some(750),
            settled_at_ms: 1_700_000_000_000_u64,
            prepare_ms: 250_u64,
            execute_ms: 1200_u64,
            queue_wait_ms: 500_u64,
            queue_wait_clamped: false,
            load_tier: "normal",
            capacity_unknowns: &["cpu_report_missing".to_string()],
            max_rss_kb: None,
            disk_delta_bytes: None,
        };

        let evidence_ref = measure::settlement_proof_evidence_ref(&fields)
            .expect("settlement proof evidence ref");

        // The evidence ref must start with the settlement prefix
        assert!(
            evidence_ref.starts_with("scheduler-settlement:"),
            "evidence ref prefix mismatch: {evidence_ref}"
        );

        // Extract the digest from the evidence ref
        let digest = evidence_ref
            .strip_prefix("scheduler-settlement:")
            .expect("settlement prefix");

        // Reconstruct the expected proof JSON and verify its digest matches
        let expected_proof = serde_json::json!({
            "workload_id": "test-workload-123",
            "status": "blocked",
            "unknowns": unknowns,
            "elapsed_ms": 1500_u64,
            "measured_cpu_ms": 750_u64,
            "settled_at_ms": 1_700_000_000_000_u64,
            "prepare_ms": 250_u64,
            "execute_ms": 1200_u64,
            "queue_wait_ms": 500_u64,
            "queue_wait_clamped": false,
            "load_tier": "normal",
            "capacity_unknowns": ["cpu_report_missing"],
            "max_rss_kb": serde_json::Value::Null,
            "disk_delta_bytes": serde_json::Value::Null,
        });
        let expected_digest = cynic_protocol::digest_json(&expected_proof).expect("digest");
        assert_eq!(
            digest, expected_digest,
            "settlement proof digest mismatch — stage timings not persisted"
        );
    }

    /// T3 (T4): a Saturated load tier must refuse admission. The gate is the
    /// pure `admit_claim_under_capacity` decision over the assessment
    /// produced by `cynic_protocol::capacity::assess_load_tier`.
    #[test]
    fn saturated_tier_skips_claim() {
        let policy = LoadPolicy::with_legacy_defaults("test-node");

        // CPU 92% > saturation threshold (85%) ⇒ Saturated. Both reports are
        // required: with memory unmeasured, assess_load_tier stays prudent
        // (Normal + unknowns) — Saturated is never asserted on half evidence.
        let cpu_saturated = assess_load_tier(
            Some(&fabricated_cpu_report(92.0)),
            Some(&fabricated_memory_report(28, 4)),
            &policy,
        );
        assert_eq!(cpu_saturated.state, LoadTier::Saturated);
        assert!(
            !admit_claim_under_capacity(&cpu_saturated),
            "saturated host must refuse claims this tick"
        );

        // Memory 26/28 GiB (92.8%) > saturation threshold (90%) ⇒ Saturated.
        let mem_saturated = assess_load_tier(
            Some(&fabricated_cpu_report(5.0)),
            Some(&fabricated_memory_report(28, 26)),
            &policy,
        );
        assert_eq!(mem_saturated.state, LoadTier::Saturated);
        assert!(
            !admit_claim_under_capacity(&mem_saturated),
            "memory-saturated host must refuse claims this tick"
        );
    }

    /// T3 (T4): Headroom and Normal tiers admit; the unmeasured path
    /// (Normal + residual unknowns) also admits — the policy default never
    /// refuses on unmeasured capacity.
    #[test]
    fn admit_claim_under_capacity_admits_standard_tiers_and_unknowns() {
        let policy = LoadPolicy::with_legacy_defaults("test-node");

        let headroom = assess_load_tier(
            Some(&fabricated_cpu_report(5.0)),
            Some(&fabricated_memory_report(28, 4)),
            &policy,
        );
        assert_eq!(headroom.state, LoadTier::Headroom);
        assert!(
            admit_claim_under_capacity(&headroom),
            "headroom host must admit claims"
        );

        let normal = assess_load_tier(
            Some(&fabricated_cpu_report(50.0)),
            Some(&fabricated_memory_report(28, 10)),
            &policy,
        );
        assert_eq!(normal.state, LoadTier::Normal);
        assert!(
            admit_claim_under_capacity(&normal),
            "normal host must admit claims"
        );

        let unmeasured = assess_load_tier(None, None, &policy);
        assert_eq!(unmeasured.state, LoadTier::Normal);
        assert!(
            !unmeasured.residual_unknowns.is_empty(),
            "unmeasured assessment must carry explicit unknowns"
        );
        assert!(
            admit_claim_under_capacity(&unmeasured),
            "policy default: unknowns proceed, never refuse on unmeasured"
        );
    }

    /// T3 (T4): the settlement proof JSON must carry the capacity gate
    /// outcome — tier string and residual unknowns — so the admission
    /// decision is auditable from persisted evidence, not logs alone.
    #[test]
    fn settlement_proof_persists_capacity_gate_outcome() {
        let status = AgentCompletionStatus::Completed;
        let fields = measure::SettlementProofFields {
            workload_id: "wl-capacity",
            status: &status,
            unknowns: &[],
            elapsed: Duration::from_millis(10),
            measured_cpu_ms: None,
            settled_at_ms: 42,
            prepare_ms: 1,
            execute_ms: 9,
            queue_wait_ms: 0,
            queue_wait_clamped: false,
            load_tier: "saturated",
            capacity_unknowns: &[
                "cpu_report_missing".to_string(),
                "memory_report_missing".to_string(),
            ],
            max_rss_kb: None,
            disk_delta_bytes: None,
        };

        let evidence_ref =
            measure::settlement_proof_evidence_ref(&fields).expect("settlement proof evidence ref");
        let digest = evidence_ref
            .strip_prefix("scheduler-settlement:")
            .expect("settlement prefix");

        let expected_proof = serde_json::json!({
            "workload_id": "wl-capacity",
            "status": "completed",
            "unknowns": Vec::<String>::new(),
            "elapsed_ms": 10_u64,
            "measured_cpu_ms": serde_json::Value::Null,
            "settled_at_ms": 42_u64,
            "prepare_ms": 1_u64,
            "execute_ms": 9_u64,
            "queue_wait_ms": 0_u64,
            "queue_wait_clamped": false,
            "load_tier": "saturated",
            "capacity_unknowns": ["cpu_report_missing", "memory_report_missing"],
            "max_rss_kb": serde_json::Value::Null,
            "disk_delta_bytes": serde_json::Value::Null,
        });
        let expected_digest = cynic_protocol::digest_json(&expected_proof).expect("digest");
        assert_eq!(
            digest, expected_digest,
            "capacity gate outcome missing from settlement proof"
        );
    }

    /// Regression: a negative raw queue wait (clock skew) must clamp to 0
    /// with `queue_wait_clamped: true` so proofs never show negative waits.
    #[test]
    fn queue_wait_clamps_negative_raw_to_zero() {
        // Simulate clock skew: first claim timestamp is BEFORE committed_at
        let (wait, clamped) = measure::compute_queue_wait_ms(900, 1000);
        assert_eq!(wait, 0, "negative raw wait must clamp to 0");
        assert!(clamped, "clamped flag must be set when skew detected");

        // Normal case: claim after commit
        let (wait, clamped) = measure::compute_queue_wait_ms(1500, 1000);
        assert_eq!(wait, 500, "normal wait must be preserved");
        assert!(!clamped, "clamped flag must be false when no skew");

        // Edge: exact same timestamp
        let (wait, clamped) = measure::compute_queue_wait_ms(1000, 1000);
        assert_eq!(wait, 0, "zero wait must stay 0");
        assert!(!clamped, "zero wait is not skew");
    }

    /// Settlement proof with no unknowns and no measured CPU still produces a
    /// valid evidence ref (None serializes to null in the JSON).
    #[test]
    fn settlement_proof_with_no_unknowns_and_no_cpu() {
        let fields = measure::SettlementProofFields {
            workload_id: QA,
            status: &AgentCompletionStatus::Completed,
            unknowns: &[],
            elapsed: Duration::ZERO,
            measured_cpu_ms: None,
            settled_at_ms: 42,
            prepare_ms: 0,
            execute_ms: 0,
            queue_wait_ms: 0,
            queue_wait_clamped: false,
            load_tier: "headroom",
            capacity_unknowns: &[],
            max_rss_kb: None,
            disk_delta_bytes: None,
        };

        let evidence_ref = measure::settlement_proof_evidence_ref(&fields)
            .expect("settlement proof evidence ref");

        let digest = evidence_ref
            .strip_prefix("scheduler-settlement:")
            .expect("settlement prefix");

        let expected_proof = serde_json::json!({
            "workload_id": QA,
            "status": "completed",
            "unknowns": Vec::<String>::new(),
            "elapsed_ms": 0_u64,
            "measured_cpu_ms": serde_json::Value::Null,
            "settled_at_ms": 42_u64,
            "prepare_ms": 0_u64,
            "execute_ms": 0_u64,
            "queue_wait_ms": 0_u64,
            "queue_wait_clamped": false,
            "load_tier": "headroom",
            "capacity_unknowns": Vec::<String>::new(),
            "max_rss_kb": serde_json::Value::Null,
            "disk_delta_bytes": serde_json::Value::Null,
        });
        let expected_digest = cynic_protocol::digest_json(&expected_proof).expect("digest");
        assert_eq!(digest, expected_digest, "empty settlement proof digest mismatch");
    }

    /// T1: settlement proof body must carry footprint fields (max_rss_kb,
    /// disk_delta_bytes) so resource consumption is auditable from persisted
    /// evidence, not logs alone.
    #[test]
    fn settlement_body_carries_footprint_fields() {
        let status = AgentCompletionStatus::Completed;
        let fields = measure::SettlementProofFields {
            workload_id: "wl-footprint",
            status: &status,
            unknowns: &[],
            elapsed: Duration::from_millis(10),
            measured_cpu_ms: None,
            settled_at_ms: 42,
            prepare_ms: 1,
            execute_ms: 9,
            queue_wait_ms: 0,
            queue_wait_clamped: false,
            load_tier: "normal",
            capacity_unknowns: &[],
            max_rss_kb: Some(1_000),
            disk_delta_bytes: Some(500),
        };

        let body = measure::settlement_proof_body(&fields);
        assert_eq!(
            body["max_rss_kb"], 1000,
            "max_rss_kb must be carried in settlement proof body"
        );
        assert_eq!(
            body["disk_delta_bytes"], 500,
            "disk_delta_bytes must be carried in settlement proof body"
        );
    }

    /// Fabricated CPU report with a known utilisation, mirroring the
    /// constructor pattern used by `cynic_protocol::capacity` tests.
    fn fabricated_cpu_report(util_pct: f64) -> cynic_protocol::cpu::CpuReport {
        cynic_protocol::cpu::CpuReport {
            schema: "cynic.host_cpu_report.v1".to_string(),
            timestamp: "test".to_string(),
            physical: cynic_protocol::cpu::CpuPhysical {
                model_name: "test".to_string(),
                logical_cpus: 16,
                cores: Some(8),
                sockets: Some(1),
                max_bogomips_or_hz: None,
            },
            volume: cynic_protocol::cpu::CpuVolume {
                governor: None,
                isolated_mask: None,
                online_cpus: 16,
                possible_cpus: 16,
            },
            usage: cynic_protocol::cpu::CpuUsage {
                user: 0,
                system: 0,
                idle: 0,
                iowait: 0,
                softirq: 0,
                steal: 0,
                util_pct: Some(util_pct),
            },
            pressure: None,
            residual_unknowns: Vec::new(),
        }
    }

    /// Fabricated memory report from total/used GiB, mirroring the
    /// constructor pattern used by `cynic_protocol::capacity` tests.
    fn fabricated_memory_report(total_gb: u64, used_gb: u64) -> cynic_protocol::memory::MemoryReport {
        cynic_protocol::memory::MemoryReport {
            schema: "cynic.host_memory_report.v1".to_string(),
            timestamp: "test".to_string(),
            physical: cynic_protocol::memory::MemoryUsage {
                total_bytes: total_gb * 1024 * 1024 * 1024,
                free_bytes: (total_gb - used_gb) * 1024 * 1024 * 1024,
                available_bytes: (total_gb - used_gb) * 1024 * 1024 * 1024,
                used_bytes: used_gb * 1024 * 1024 * 1024,
                reclaimable_bytes: 0,
            },
            volume: cynic_protocol::memory::MemoryVolume {
                page_size_bytes: 4096,
                swappiness: None,
                hugepages_reserved: None,
                hugepage_size_bytes: None,
            },
            swap: cynic_protocol::memory::MemoryUsage {
                total_bytes: 0,
                free_bytes: 0,
                available_bytes: 0,
                used_bytes: 0,
                reclaimable_bytes: 0,
            },
            pressure: None,
            meminfo_headline: None,
            residual_unknowns: Vec::new(),
        }
    }

    fn scheduler_configs() -> (
        crate::config::NodeControlConfig,
        crate::config::BuildSchedulerConfig,
    ) {
        use std::path::PathBuf;
        let control = crate::config::NodeControlConfig {
            kernel_url: String::new(),
            node_id: "node-1".to_string(),
            signing_key_path: PathBuf::from("/tmp/x"),
            enrollment_request_path: PathBuf::from("/tmp/x"),
            enrollment_grant_path: PathBuf::from("/tmp/x"),
            enrollment_authority_public_key_path: PathBuf::from("/tmp/x"),
            enrollment_token_path: PathBuf::from("/tmp/x"),
            enrollment_seed_id: String::new(),
            enrollment_role: String::new(),
            agent_workload_state_root: PathBuf::from("/tmp/x"),
            runtime_artifact_inbox_root: PathBuf::from("/tmp/x"),
            runtime_artifact_max_bytes: 0,
            runtime_artifact_timeout_secs: 0,
            enrollment_requested_capabilities: Vec::new(),
            enrollment_request_ttl_secs: 0,
            enrollment_renewal_window_secs: 0,
            interval_secs: 0,
            timeout_secs: 0,
            workload_delivery_lease_ttl_secs: 0,
            observed_capabilities: Vec::new(),
            observed_services: Vec::new(),
            registry_path: PathBuf::from("/tmp/x"),
            build_scheduler: None,
            builder_listen_addr: "127.0.0.1:1".parse().expect("addr"),
            workspace_root: PathBuf::from("/tmp/x"),
            membership_interval_secs: 0,
        };
        let scheduler = crate::config::BuildSchedulerConfig {
            registry_path: PathBuf::from("/tmp/x"),
            repository_id: String::new(),
            transport_uri: None,
            interval_secs: 0,
            claim_ttl_secs: 60,
            command_timeout_secs: 900,
            sandbox_bwrap: false,
            shared_target_root: None,
            stable_workspace_root: None,
            release_publication: None,
            build_jobs: None,
            max_rss_cap_mb: None,
            sccache: None,
            sccache_measured_hit_rate: None,
        };
        (control, scheduler)
    }

    // ── T1 fix round 1 tests ─────────────────────────────────────────────
    // These tests pin correct footprint measurement semantics:
    //   1. measure::measure_max_rss_kb() uses getrusage(RUSAGE_CHILDREN) (the build
    //      child tree peak), not /proc/self/status (the scheduler's own RSS).
    //   2. disk_delta_bytes uses du-style target-dir growth (recursive size
    //      of effective target dir before/after the execute window), not
    //      statvfs free-space deltas which were erratic under multi-agent
    //      load because neighbors' writes polluted the measurement.

    /// RED: disk_delta must see writes to the effective target dir. Writing
    /// a known 20 MB into the target dir inside the measured window must
    /// produce a delta in the right order of magnitude (tens of MB), not
    /// near zero. Uses du-style target-dir growth: the difference between
    /// the directory size before and after the write.
    ///
    /// NOTE: This measurement is approximate under concurrency because
    /// multiple builds may share the same target dir. The delta includes
    /// any writes by other processes to that directory.
    #[test]
    fn disk_delta_sees_writes_to_effective_target_dir() {
        use std::io::Write;

        let workspace_root = std::env::temp_dir().join(format!(
            "cynic-delta-{}-{}",
            std::process::id(),
            QUEUE_SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&workspace_root).expect("create workspace root");

        let target_dir = measure::effective_target_dir(None, &workspace_root, &[]);
        std::fs::create_dir_all(&target_dir).expect("create target dir");

        let size_before = measure::measure_target_dir_size_bytes(&target_dir).expect("measure before");

        let known_bytes = 20 * 1024 * 1024u64;
        let file_path = target_dir.join("known_write.bin");
        let buf = vec![0u8; 1024 * 1024];
        let mut file = std::fs::File::create(&file_path).expect("create file");
        for _ in 0..(known_bytes / (1024 * 1024)) {
            file.write_all(&buf).expect("write");
        }
        drop(file);

        let size_after = measure::measure_target_dir_size_bytes(&target_dir).expect("measure after");
        let disk_delta_bytes = size_after as i64 - size_before as i64;

        assert!(
            disk_delta_bytes >= (known_bytes as i64) / 2,
            "disk_delta must see the 20 MB write, got {disk_delta_bytes} bytes"
        );

        let _ = std::fs::remove_file(&file_path);
        let _ = std::fs::remove_dir_all(&workspace_root);
    }

    /// RED: measure_max_rss_kb must reflect the build child's memory, not
    /// the scheduler's own RSS. A child allocating ~50 MB must produce a
    /// measurement in the right order of magnitude (tens of MB), not ~768 KB.
    #[test]
    fn measure_max_rss_kb_reflects_child_memory_not_self() {
        use std::process::Command;
        // Spawn a child that allocates and touches ~50 MB.
        let child = Command::new("sh")
            .arg("-c")
            .arg("python3 -c \"import ctypes; ctypes.string_at(50 * 1024 * 1024)\" 2>/dev/null || dd if=/dev/zero bs=1M count=50 2>/dev/null | wc -c")
            .output()
            .expect("spawn child");
        assert!(child.status.success(), "child must succeed");
        // The child has exited; RUSAGE_CHILDREN now includes it.
        // The current measure::measure_max_rss_kb() reads /proc/self/status which
        // is the test process's own RSS (a few MB at most), NOT the child's.
        // After the fix, this function will use getrusage(RUSAGE_CHILDREN)
        // and must report the child's peak (tens of MB).
        let measured = measure::measure_max_rss_kb().expect("measurement must succeed");
        // The child touched ~50 MB. The measurement must be at least 10 MB
        // (order of magnitude). The current code returns ~768 KB (the test
        // binary's own RSS), which fails this assertion.
        assert!(
            measured > 10_000,
            "measure_max_rss_kb must reflect child memory (tens of MB), got {measured} kB"
        );
    }

    /// Regression: measure_target_dir_size_bytes must reflect the target
    /// dir's content, not the filesystem's free space. Adding a file to
    /// the target dir must increase the measured size.
    #[test]
    fn measure_target_dir_size_bytes_reflects_content() {
        let tmp = std::env::temp_dir().join(format!(
            "cynic-disk-test-{}-{}",
            std::process::id(),
            QUEUE_SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&tmp).expect("create tmp dir");
        // Write a known 20 MB file to the target dir.
        let known_bytes = 20 * 1024 * 1024u64;
        let file_path = tmp.join("known_write.bin");
        let buf = vec![0u8; 1024 * 1024];
        use std::io::Write;
        let mut file = std::fs::File::create(&file_path).expect("create file");
        for _ in 0..(known_bytes / (1024 * 1024)) {
            file.write_all(&buf).expect("write");
        }
        drop(file);
        let size_before = measure::measure_target_dir_size_bytes(&tmp).expect("measure before");
        std::fs::remove_file(&file_path).expect("remove file");
        let size_after = measure::measure_target_dir_size_bytes(&tmp).expect("measure after");
        let delta = size_before as i64 - size_after as i64;
        assert!(
            delta >= (known_bytes as i64) / 2,
            "target dir size must see the 20 MB write, got {delta} bytes"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Regression: an EXPIRED lease must be reclaimable, a LIVE lease must be
    /// skipped. Before the fix, every Claimed record was skipped and the
    /// recovery path in run_iteration was unreachable — expired claims
    /// stranded forever.
    #[test]
    fn expired_claim_reclaims_live_claim_skips() {
        use cynic_protocol::LeaseGuard;
        use cynic_workload::AgentWorkloadClaim;

        let store = queue_store();
        let admitted = admit_build_check(
            QA, "cynic-host", "cynic-builder", "node-1", "agent-1", 100, &store,
        )
        .expect("admit");
        let (control, scheduler) = scheduler_configs();

        let mut expired = admitted.record.clone();
        expired.state = cynic_workload::AgentWorkloadState::Claimed;
        expired.last_fencing_token = 1;
        expired.claim = Some(AgentWorkloadClaim {
            guard: LeaseGuard {
                agent_id: "agent-1".to_string(),
                target: expired.envelope.workload_id.clone(),
                lease_id: "old-lease".to_string(),
                fencing_token: 1,
            },
            expires_at_ms: 1,
            cancellation_generation: 0,
            max_lease_window_ms: 900_000,
            granted_at_ms: 1,
        });
        let forged = store
            .compare_and_swap(
                &expired.envelope.workload_id.clone(),
                Some(admitted.revision),
                &expired,
                200,
            )
            .expect("forge expired claim");

        let reclaimed = lifecycle::claim_if_available(&control, &scheduler, &store, &forged)
            .expect("no store error")
            .expect("expired claim must reclaim");
        assert_eq!(
            reclaimed.record.state,
            cynic_workload::AgentWorkloadState::Claimed
        );
        assert_eq!(reclaimed.record.last_fencing_token, 2);

        let live = store
            .read(&reclaimed.record.envelope.workload_id)
            .expect("read")
            .expect("present");
        let skipped = lifecycle::claim_if_available(&control, &scheduler, &store, &live)
            .expect("no store error");
        assert!(skipped.is_none(), "live claim must be skipped");
    }

    // ── H2: RSS cap gate tests (against the single T6 authority) ──────────

    /// The gate must refuse admission when projected RSS exceeds the cap.
    #[test]
    fn rss_cap_gate_refuses_when_projected_exceeds_cap() {
        assert!(
            !decide_admit_parallel_slot(2048, 4096),
            "projected RSS (4096 MB) exceeds cap (2048 MB) — must refuse"
        );
    }

    /// The gate must admit when projected RSS is under the cap.
    #[test]
    fn rss_cap_gate_admits_when_projected_under_cap() {
        assert!(
            decide_admit_parallel_slot(2048, 1024),
            "projected RSS (1024 MB) under cap (2048 MB) — must admit"
        );
    }

    /// The gate must admit when projected RSS equals the cap boundary.
    #[test]
    fn rss_cap_gate_admits_at_exact_cap_boundary() {
        assert!(
            decide_admit_parallel_slot(2048, 2048),
            "projected RSS equals cap — must admit (boundary inclusive)"
        );
    }

    /// Projection: build_jobs=None must use JOBS=1 (single rustc process).
    #[test]
    fn projected_rss_uses_single_job_when_build_jobs_none() {
        let projected = projected_rss_mb(None);
        assert_eq!(
            projected, RSS_PER_JOB_MB,
            "build_jobs=None must project as 1 job"
        );
    }

    /// Projection: build_jobs=4 must project as 4× the per-job estimate.
    #[test]
    fn projected_rss_scales_with_build_jobs() {
        let projected = projected_rss_mb(Some(4));
        assert_eq!(
            projected,
            4 * RSS_PER_JOB_MB,
            "build_jobs=4 must project as 4× RSS_PER_JOB_MB"
        );
    }

    // ── T3 fix round 1: grounded constant + admission-path integration ──

    /// The grounded per-job RSS constant must reflect the measured peak RSS
    /// on this builder (infra/build-policy.json: peak_rss_mib.single_job=1166).
    /// A single job must project to at least 1166 MB.
    #[test]
    fn projected_rss_single_job_at_least_measured_peak() {
        let projected = projected_rss_mb(Some(1));
        assert!(
            projected >= 1166,
            "projected_rss_mb(Some(1)) = {} must be >= 1166 (measured peak_rss_mib.single_job)",
            projected
        );
    }

    /// Integration at the admission-path level: when projected RSS exceeds
    /// the cap, the combined admission decision must refuse.
    #[test]
    fn admission_path_refuses_when_projected_rss_exceeds_cap() {
        let scheduler = BuildSchedulerConfig {
            registry_path: PathBuf::from("/tmp/x"),
            repository_id: "test".to_string(),
            transport_uri: None,
            interval_secs: 5,
            claim_ttl_secs: 60,
            command_timeout_secs: 900,
            sandbox_bwrap: false,
            shared_target_root: None,
            stable_workspace_root: None,
            release_publication: None,
            build_jobs: Some(2),
            max_rss_cap_mb: Some(100),
            sccache: None,
            sccache_measured_hit_rate: None,
        };
        let capacity = CapacityAssessmentV1 {
            schema: cynic_protocol::capacity::CAPACITY_ASSESSMENT_SCHEMA,
            state: LoadTier::Headroom,
            cpu_util_pct: None,
            mem_used_pct: None,
            available_ram_bytes: 0,
            policy_id: "build-scheduler".to_string(),
            residual_unknowns: vec![],
        };
        assert!(
            !should_admit_build(&capacity, &scheduler).admitted,
            "admission must refuse when projected RSS exceeds cap"
        );
    }

    /// Integration at the admission-path level: when projected RSS is under
    /// the cap, the combined admission decision must admit.
    #[test]
    fn admission_path_admits_when_projected_rss_under_cap() {
        let scheduler = BuildSchedulerConfig {
            registry_path: PathBuf::from("/tmp/x"),
            repository_id: "test".to_string(),
            transport_uri: None,
            interval_secs: 5,
            claim_ttl_secs: 60,
            command_timeout_secs: 900,
            sandbox_bwrap: false,
            shared_target_root: None,
            stable_workspace_root: None,
            release_publication: None,
            build_jobs: Some(1),
            max_rss_cap_mb: Some(4096),
            sccache: None,
            sccache_measured_hit_rate: None,
        };
        let capacity = CapacityAssessmentV1 {
            schema: cynic_protocol::capacity::CAPACITY_ASSESSMENT_SCHEMA,
            state: LoadTier::Headroom,
            cpu_util_pct: None,
            mem_used_pct: None,
            available_ram_bytes: 0,
            policy_id: "build-scheduler".to_string(),
            residual_unknowns: vec![],
        };
        assert!(
            should_admit_build(&capacity, &scheduler).admitted,
            "admission must admit when projected RSS is under cap"
        );
    }

    // ── T6: predicate wiring tests ────────────────────────────────────────

    fn headroom_capacity() -> CapacityAssessmentV1 {
        CapacityAssessmentV1 {
            schema: cynic_protocol::capacity::CAPACITY_ASSESSMENT_SCHEMA,
            state: LoadTier::Headroom,
            cpu_util_pct: None,
            mem_used_pct: None,
            available_ram_bytes: 0,
            policy_id: "build-scheduler".to_string(),
            residual_unknowns: vec![],
        }
    }

    /// H1: `decide_reuse_stable_checkout` is consulted at the admission
    /// decision point. When the stable root is configured the predicate
    /// returns true and the decision records reuse; when absent, false.
    #[test]
    fn admission_consults_reuse_stable_checkout_predicate() {
        let capacity = headroom_capacity();
        let mut scheduler = scheduler_configs().1;
        scheduler.stable_workspace_root = Some(PathBuf::from("/tmp/stable"));
        let decision = should_admit_build(&capacity, &scheduler);
        assert!(decision.admitted);
        assert!(
            decision.stable_checkout_reuse,
            "stable root configured ⇒ predicate must report reuse"
        );
        scheduler.stable_workspace_root = None;
        let decision = should_admit_build(&capacity, &scheduler);
        assert!(decision.admitted);
        assert!(
            !decision.stable_checkout_reuse,
            "stable root absent ⇒ predicate must report no reuse"
        );
    }

    /// H2: `decide_admit_parallel_slot` gates admission. When no cap is
    /// configured the predicate's `RSS_CAP_MB` is the default; projected
    /// RSS above it must refuse.
    #[test]
    fn admission_uses_predicate_rss_cap_as_default() {
        let capacity = headroom_capacity();
        let mut scheduler = scheduler_configs().1;
        scheduler.build_jobs = Some(10);
        scheduler.max_rss_cap_mb = None;
        let decision = should_admit_build(&capacity, &scheduler);
        assert!(
            !decision.admitted,
            "projected RSS {} MB exceeds default cap {} MB — must refuse",
            projected_rss_mb(scheduler.build_jobs),
            RSS_CAP_MB,
        );
        assert!(!decision.parallel_slot_admitted);
    }

    /// H2: `decide_admit_parallel_slot` boundary — projected == cap admits.
    #[test]
    fn admission_parallel_slot_boundary_inclusive() {
        let capacity = headroom_capacity();
        let mut scheduler = scheduler_configs().1;
        scheduler.build_jobs = Some(1);
        scheduler.max_rss_cap_mb = Some(projected_rss_mb(scheduler.build_jobs));
        let decision = should_admit_build(&capacity, &scheduler);
        assert!(decision.admitted);
        assert!(decision.parallel_slot_admitted);
    }

    /// H3: `decide_require_sccache` is consulted when a measured hit rate
    /// is present. At or above the threshold ⇒ required; below ⇒ not.
    #[test]
    fn admission_consults_require_sccache_predicate() {
        let capacity = headroom_capacity();
        let mut scheduler = scheduler_configs().1;
        scheduler.sccache_measured_hit_rate = Some(0.90);
        let decision = should_admit_build(&capacity, &scheduler);
        assert!(decision.admitted);
        assert!(
            decision.sccache_required,
            "hit rate {} >= {} ⇒ sccache required",
            0.90,
            SCCACHE_ADOPT_THRESHOLD,
        );
        scheduler.sccache_measured_hit_rate = Some(0.10);
        let decision = should_admit_build(&capacity, &scheduler);
        assert!(decision.admitted);
        assert!(
            !decision.sccache_required,
            "hit rate {} < {} ⇒ sccache not required",
            0.10,
            SCCACHE_ADOPT_THRESHOLD,
        );
    }

    /// H3: when no measured hit rate is present the sccache predicate is
    /// not consulted and `sccache_required` stays false.
    #[test]
    fn admission_sccache_not_consulted_when_unmeasured() {
        let capacity = headroom_capacity();
        let scheduler = scheduler_configs().1;
        assert!(scheduler.sccache_measured_hit_rate.is_none());
        let decision = should_admit_build(&capacity, &scheduler);
        assert!(decision.admitted);
        assert!(
            !decision.sccache_required,
            "unmeasured hit rate ⇒ predicate not consulted"
        );
    }
}
