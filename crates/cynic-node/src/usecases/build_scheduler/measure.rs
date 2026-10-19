//! Footprint measurement and proof persistence for the build scheduler.
//!
//! Pure measures (RSS, target-dir size, queue wait) plus the proof writers
//! (renewal, cancellation, settlement, recovery). The execution orchestrator
//! (`execute`) calls in; nothing here calls back.

use std::time::Duration;

use cynic_protocol::AgentCompletionStatus;

use super::prepare;

pub(crate) fn persist_renewal_proof(
    workload_id: &str,
    guard: &cynic_protocol::LeaseGuard,
    renewal_count: u64,
) {
    if renewal_count == 0 {
        return;
    }
    let proof = serde_json::json!({
        "workload_id": workload_id,
        "lease_id": guard.lease_id.as_str(),
        "fencing_token": guard.fencing_token,
        "successful_renewals": renewal_count,
    });
    let evidence_ref = cynic_protocol::digest_json(&proof)
        .map(|digest| format!("scheduler-claim-renewal:{digest}"))
        .unwrap_or_else(|_| format!("scheduler-claim-renewal:{workload_id}"));
    if let Err(error) = crate::adapters::agent_execution_runtime_observer::persist_capabilities(
        &["scheduler_claim_renewal"],
        &[evidence_ref],
    ) {
        tracing::warn!(%error, workload_id, "scheduler renewal proof read-back unavailable");
    }
}

pub(crate) fn persist_cancellation_proof(workload_id: &str, revision: u64) {
    let proof = serde_json::json!({
        "workload_id": workload_id,
        "store_revision": revision,
        "transition": "cancel_requested_to_terminal_acknowledged",
    });
    let evidence_ref = cynic_protocol::digest_json(&proof)
        .map(|digest| format!("scheduler-cancellation-delivery:{digest}"))
        .unwrap_or_else(|_| format!("scheduler-cancellation-delivery:{workload_id}"));
    if let Err(error) = crate::adapters::agent_execution_runtime_observer::persist_capabilities(
        &["scheduler_cancellation_delivery"],
        &[evidence_ref],
    ) {
        tracing::warn!(%error, workload_id, "scheduler cancellation proof read-back unavailable");
    }
}

/// Compute queue wait duration from first claim time and commit time.
/// Returns `(queue_wait_ms, clamped)` where `clamped` is true when clock
/// skew produced a negative raw wait (proofs never show negative durations).
pub(crate) fn compute_queue_wait_ms(first_claim_ms: u64, committed_at_ms: u64) -> (u64, bool) {
    let raw = first_claim_ms as i64 - committed_at_ms as i64;
    if raw < 0 {
        (0, true)
    } else {
        (raw as u64, false)
    }
}

/// Measure peak resident set size of the build child process tree via
/// `getrusage(RUSAGE_CHILDREN)`. Returns `ru_maxrss` in kB (Linux).
/// This reflects the largest waited-for descendant (rustc), not the
/// scheduler's own RSS. `None` when `getrusage` is unavailable.
/// Safe wrapper (`nix`) — the workspace forbids `unsafe` blocks.
pub(crate) fn measure_max_rss_kb() -> Option<u64> {
    let usage =
        nix::sys::resource::getrusage(nix::sys::resource::UsageWho::RUSAGE_CHILDREN).ok()?;
    let peak_kb = usage.max_rss().max(0) as u64;
    // On Linux, ru_maxrss is in kilobytes.
    Some(peak_kb)
}

/// Measure the total size of all files in `path` recursively (du-style).
/// Returns `None` when the path doesn't exist or can't be read.
///
/// This replaces the old statvfs-based free-space delta, which was erratic
/// under multi-agent load because neighbors' writes polluted the measurement.
/// Target-dir growth is approximate under concurrency: multiple builds may
/// share the same target dir, so the delta includes any writes by other
/// processes to that directory.
pub(crate) fn measure_target_dir_size_bytes(path: &std::path::Path) -> Option<u64> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if metadata.is_file() {
        return Some(metadata.len());
    }
    if !metadata.is_dir() {
        return None;
    }
    let mut total = 0u64;
    let entries = std::fs::read_dir(path).ok()?;
    for entry in entries {
        let entry = entry.ok()?;
        let size = measure_target_dir_size_bytes(&entry.path())?;
        total += size;
    }
    Some(total)
}

/// Resolve the effective CARGO_TARGET_DIR for a build. When
/// `shared_target_root` is set, the target dir is `<root>/<profile>`.
/// Otherwise, fall back to the workspace root's `target` dir (cargo's
/// default). Pure.
pub(crate) fn effective_target_dir(
    shared_target_root: Option<&std::path::Path>,
    workspace_root: &std::path::Path,
    argv: &[String],
) -> std::path::PathBuf {
    match shared_target_root {
        Some(root) => root.join(prepare::profile_for_argv(argv)),
        None => workspace_root.join("target"),
    }
}

/// Fields persisted into the settlement proof JSON. Additive across the
/// scheduler-truths tasks: T1 reasons/timings, T2 stage and queue-wait
/// timings, T3 capacity gate outcome, T4 footprint (max RSS + disk delta).
pub(crate) struct SettlementProofFields<'a> {
    pub workload_id: &'a str,
    pub status: &'a AgentCompletionStatus,
    pub unknowns: &'a [String],
    pub elapsed: Duration,
    pub measured_cpu_ms: Option<u64>,
    pub settled_at_ms: u64,
    pub prepare_ms: u64,
    pub execute_ms: u64,
    pub queue_wait_ms: u64,
    pub queue_wait_clamped: bool,
    pub load_tier: &'a str,
    pub capacity_unknowns: &'a [String],
    pub max_rss_kb: Option<u64>,
    pub disk_delta_bytes: Option<i64>,
}

/// Build the canonical settlement proof body as JSON. Pure: the same facts
/// feed both the evidence digest and the durable sidecar.
pub(crate) fn settlement_proof_body(fields: &SettlementProofFields<'_>) -> serde_json::Value {
    serde_json::json!({
        "workload_id": fields.workload_id,
        "status": fields.status,
        "unknowns": fields.unknowns,
        "elapsed_ms": fields.elapsed.as_millis(),
        "measured_cpu_ms": fields.measured_cpu_ms,
        "settled_at_ms": fields.settled_at_ms,
        "prepare_ms": fields.prepare_ms,
        "execute_ms": fields.execute_ms,
        "queue_wait_ms": fields.queue_wait_ms,
        "queue_wait_clamped": fields.queue_wait_clamped,
        "load_tier": fields.load_tier,
        "capacity_unknowns": fields.capacity_unknowns,
        "max_rss_kb": fields.max_rss_kb,
        "disk_delta_bytes": fields.disk_delta_bytes,
    })
}

/// Build the canonical settlement proof evidence ref. Pure: serializes the
/// settlement facts to canonical JSON and digests them with SHA3-512.
pub(crate) fn settlement_proof_evidence_ref(
    fields: &SettlementProofFields<'_>,
) -> Result<String, serde_json::Error> {
    let proof = settlement_proof_body(fields);
    cynic_protocol::digest_json(&proof)
        .map(|digest| format!("scheduler-settlement:{digest}"))
}

pub(crate) fn persist_settlement_proof(
    store: &dyn crate::ports::agent_workload_store_port::AgentWorkloadStorePort,
    fields: &SettlementProofFields<'_>,
) {
    let proof = settlement_proof_body(fields);
    // Single digest path: the pure helper above is the only constructor.
    // (Previously duplicated inline, leaving the helper test-only and the
    // lib red under deny(dead_code).)
    let evidence_ref = settlement_proof_evidence_ref(fields)
        .unwrap_or_else(|_| format!("scheduler-settlement:{}", fields.workload_id));
    if let Err(error) = crate::adapters::agent_execution_runtime_observer::persist_capabilities(
        &["scheduler_settlement"],
        &[evidence_ref],
    ) {
        tracing::warn!(%error, workload_id = %fields.workload_id, "scheduler settlement proof read-back unavailable");
    }
    // Durable body next to the record it explains: the digest alone cannot
    // answer WHY a build died. Best-effort — never fails the settlement.
    let file_name = format!("settlement-{}.json", fields.settled_at_ms);
    if let Err(error) = store.append_evidence(fields.workload_id, &file_name, &proof) {
        tracing::warn!(?error, workload_id = %fields.workload_id, "scheduler settlement body unavailable");
    }
}

pub(crate) fn persist_recovery_proof(workload_id: &str, prior_revision: u64, new_revision: u64) {
    let proof = serde_json::json!({
        "workload_id": workload_id,
        "prior_store_revision": prior_revision,
        "new_store_revision": new_revision,
        "transition": "expired_claim_reclaimed_with_new_fence",
    });
    let evidence_ref = cynic_protocol::digest_json(&proof)
        .map(|digest| format!("scheduler-crash-recovery:{digest}"))
        .unwrap_or_else(|_| format!("scheduler-crash-recovery:{workload_id}"));
    if let Err(error) = crate::adapters::agent_execution_runtime_observer::persist_capabilities(
        &["scheduler_crash_recovery"],
        &[evidence_ref],
    ) {
        tracing::warn!(%error, workload_id, "scheduler recovery proof read-back unavailable");
    }
}
