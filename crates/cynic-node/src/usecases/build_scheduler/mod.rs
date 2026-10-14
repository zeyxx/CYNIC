mod evidence;
mod lifecycle;
mod prepare;
mod release_publication;
mod source_prepare;

use std::time::{Duration, Instant};

use cynic_build::{QueuedCandidate, canonical_build_action_from_argv, plan_build_queue};
use cynic_host::adapters::local_capacity::LocalCapacityAdapter;
use cynic_host::adapters::local_evidence_store::LocalContentAddressedEvidenceStore;
use cynic_protocol::capacity::{
    CapacityAssessmentV1, LoadPolicy, LoadTier, assess_load_tier,
};
use cynic_protocol::AgentCompletionStatus;
use cynic_workload::{AgentWorkloadState, DurableAgentWorkloadRecord, admit_agent_workload};
use tokio_util::sync::CancellationToken;

use crate::{
    adapters::{
        local_agent_workload_store::LocalAgentWorkloadStore,
        tokio_build_execution::TokioBuildExecution,
    },
    config::{BuildSchedulerConfig, NodeControlConfig},
    ports::{
        agent_workload_store_port::AgentWorkloadStorePort,
        build_scheduler_port::{BuildExecutionError, BuildExecutionPort},
    },
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
    if !admit_claim_under_capacity(&capacity) {
        tracing::info!(
            tier = %load_tier,
            "build scheduler tick skipped: host capacity saturated"
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
            persist_cancellation_proof(&current.record.envelope.workload_id, current.revision);
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
            persist_recovery_proof(
                &claimed.record.envelope.workload_id,
                current.revision,
                claimed.revision,
            );
        }
        execute_claimed(
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

async fn execute_claimed(
    control: &NodeControlConfig,
    scheduler: &BuildSchedulerConfig,
    store: &LocalAgentWorkloadStore,
    claimed: DurableAgentWorkloadRecord,
    shutdown: &CancellationToken,
    load_tier: &str,
    capacity_unknowns: &[String],
) -> Result<(), String> {
    let guard = claimed
        .record
        .claim
        .as_ref()
        .map(|claim| claim.guard.clone())
        .ok_or_else(|| "build_scheduler_claim_missing".to_string())?;

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let admission = admit_agent_workload(&claimed.record.envelope);
    if !admission.admitted() {
        return Err(format!("admission_gate_refused:{:?}", admission));
    }
    if claimed
        .record
        .claim
        .as_ref()
        .is_none_or(|claim| claim.expires_at_ms <= now_ms)
    {
        return Err("admission_gate_refused:claim_expired".to_string());
    }

    let started = Instant::now();
    // cpu_ms measurement scope: one meter per workload execution. Marks
    // bracket the build; the delta lands in the completion envelope's
    // consumed_budget.cpu_ms (None stays None when sysinfo cannot sample).
    let mut cpu_meter = cynic_host::adapters::cpu_usage_meter::CpuUsageMeter::new();
    let cpu_start = cpu_meter.mark();
    let execution_cancel = CancellationToken::new();
    let monitor = tokio::spawn(lifecycle::monitor_claim(
        store.clone(),
        claimed.record.envelope.workload_id.clone(),
        guard.clone(),
        scheduler.claim_ttl_secs,
        execution_cancel.clone(),
        shutdown.clone(),
    ));
    let prepare_start = Instant::now();
    let prepared = match prepare::prepare(
        control,
        scheduler,
        store,
        &claimed,
        &guard,
        &execution_cancel,
    )
    .await
    {
        Ok(prepared) => prepared,
        Err(error) => {
            tracing::warn!(%error, workload_id = %claimed.record.envelope.workload_id, "build preparation failed");
            execution_cancel.cancel();
            let renewal_count = monitor.await.unwrap_or(0);
            persist_renewal_proof(&claimed.record.envelope.workload_id, &guard, renewal_count);
            lifecycle::finish_blocked(store, &claimed.record.envelope.workload_id, &guard, error)?;
            return Ok(());
        }
    };
    let prepare_ms = prepare_start.elapsed().as_millis() as u64;
    let execute_start = Instant::now();
    let execution = TokioBuildExecution
        .execute(&prepared.execution, &execution_cancel)
        .await;
    let execute_ms = execute_start.elapsed().as_millis() as u64;
    let workload_id = &prepared.durable.record.envelope.workload_id;
    let after_execution = store
        .read(workload_id)
        .map_err(|error| error.code)?
        .ok_or_else(|| "build_scheduler_workload_disappeared".to_string())?;
    if after_execution.record.state == AgentWorkloadState::CancelRequested {
        execution_cancel.cancel();
        let renewal_count = monitor.await.unwrap_or(0);
        persist_renewal_proof(workload_id, &guard, renewal_count);
        lifecycle::acknowledge_cancelled(
            store,
            &after_execution,
            "workload_cancelled_during_execution",
        )?;
        persist_cancellation_proof(workload_id, after_execution.revision);
        return Ok(());
    }
    let evidence_store =
        LocalContentAddressedEvidenceStore::new(control.agent_workload_state_root.join("evidence"));
    let memory_observation_unknown = matches!(
        &execution,
        Ok(read_back) if read_back.peak_memory_bytes.is_none()
    );
    let (status, evidence_refs, mut unknowns) = match execution {
        Ok(read_back) if read_back.exit_code == Some(0) => {
            let evidence = evidence::persist_success_proofs(
                &control.node_id,
                workload_id,
                &after_execution.record.envelope.delegation.expected_proofs,
                &prepared.workspace,
                &read_back,
                &after_execution.record.envelope.action.argv,
                lifecycle::current_unix_ms(),
                &prepared.provenance,
                &evidence_store,
            );
            match evidence {
                Ok(mut evidence_refs) => {
                    match evidence::persist_execution_provenance(
                        &control.node_id,
                        workload_id,
                        &scheduler.repository_id,
                        &after_execution.record.envelope.delegation.delegation_id,
                        &after_execution.record.envelope.delegation.agent_id,
                        &guard.lease_id,
                        guard.fencing_token,
                        &after_execution.record_digest,
                        &prepared.workspace,
                        &read_back,
                        &after_execution.record.envelope.action.argv,
                        lifecycle::current_unix_ms(),
                        prepared.provenance.clone(),
                        &evidence_store,
                    ) {
                        Ok(reference) => {
                            evidence_refs.push(reference);
                            let publication = release_publication::publish(
                                &control.node_id,
                                &after_execution.record.envelope,
                                &prepared.workspace,
                                &evidence_refs,
                                scheduler.release_publication.as_ref(),
                                lifecycle::current_unix_ms(),
                                &prepared.provenance,
                                &evidence_store,
                            );
                            let publication_result = tokio::select! {
                                result = publication => result,
                                () = execution_cancel.cancelled() => {
                                    Err("release_publication_cancelled".to_string())
                                }
                            };
                            let publication_error = match publication_result {
                                Ok(Some(reference)) => {
                                    evidence_refs.push(reference);
                                    None
                                }
                                Ok(None) => None,
                                Err(error) => Some(error),
                            };
                            if let Some(error) = publication_error {
                                (AgentCompletionStatus::Degraded, evidence_refs, vec![error])
                            } else if after_execution
                                .record
                                .envelope
                                .delegation
                                .expected_proofs
                                .iter()
                                .all(|proof| {
                                    evidence_refs
                                        .iter()
                                        .any(|reference| &reference.kind == proof)
                                })
                            {
                                (AgentCompletionStatus::Completed, evidence_refs, Vec::new())
                            } else {
                                (
                                    AgentCompletionStatus::Degraded,
                                    evidence_refs,
                                    vec!["build_scheduler_required_proof_missing".to_string()],
                                )
                            }
                        }
                        Err(error) => (AgentCompletionStatus::Blocked, evidence_refs, vec![error]),
                    }
                }
                Err(error) => (AgentCompletionStatus::Blocked, Vec::new(), vec![error]),
            }
        }
        Ok(read_back) => {
            let evidence_ref = evidence::persist_execution_failure(
                &control.node_id,
                workload_id,
                &read_back,
                lifecycle::current_unix_ms(),
                prepared.provenance,
                &evidence_store,
            )
            .ok()
            .into_iter()
            .collect();
            (
                AgentCompletionStatus::Failed,
                evidence_ref,
                vec![format!("build_exit_code:{:?}", read_back.exit_code)],
            )
        }
        Err(error) => {
            let unknown = match error {
                BuildExecutionError::Cancelled => "build_execution_cancelled".to_string(),
                BuildExecutionError::Timeout => "build_execution_timeout".to_string(),
                BuildExecutionError::Invalid(error) => error,
                BuildExecutionError::Spawn(error) => format!("build_execution_spawn:{error}"),
            };
            (AgentCompletionStatus::Blocked, Vec::new(), vec![unknown])
        }
    };
    if memory_observation_unknown {
        unknowns.push("build_peak_memory_unobserved".to_string());
    }
    execution_cancel.cancel();
    let renewal_count = monitor.await.unwrap_or(0);
    // Close the cpu measurement scope: sample after the build, delta into
    // the completion envelope. None = unmeasured (explicit unknown).
    let cpu_end = cpu_meter.mark();
    let measured_cpu_ms = cpu_meter.cpu_ms_between(&cpu_start, &cpu_end);
    let latest = store
        .read(workload_id)
        .map_err(|error| error.code)?
        .ok_or_else(|| "build_scheduler_workload_disappeared".to_string())?;
    if latest.record.state == AgentWorkloadState::CancelRequested {
        lifecycle::acknowledge_cancelled(store, &latest, "workload_cancelled_during_proof")?;
        persist_cancellation_proof(workload_id, latest.revision);
        Ok(())
    } else {
        // Must use latest.record.claim's guard so the lease and fencing token
        // strictly match the latest durable revision after claim renewals!
        let active_guard = latest
            .record
            .claim
            .as_ref()
            .map(|c| c.guard.clone())
            .unwrap_or_else(|| guard.clone());

        let settlement_elapsed = started.elapsed();
        let settlement_unknowns = unknowns.clone();
        // Queue wait: first claim time minus durable commit time. Uses
        // latest.committed_at_ms (immutable across revisions) and the claim's
        // granted_at_ms (fixed at initial claim, unchanged by renewals).
        let first_claim_ms = latest
            .record
            .claim
            .as_ref()
            .map(|c| c.granted_at_ms)
            .unwrap_or(0);
        let (queue_wait_ms, queue_wait_clamped) =
            compute_queue_wait_ms(first_claim_ms, latest.committed_at_ms);
        let _receipt = match lifecycle::finish(
            store,
            &latest,
            &active_guard,
            status,
            evidence_refs.clone(),
            unknowns,
            settlement_elapsed,
            measured_cpu_ms,
        ) {
            Ok(receipt) => {
                tracing::info!(
                    workload_id,
                    settled_at_ms = receipt.settled_at_ms,
                    final_record_digest = %receipt.final_record_digest,
                    "hermetic parcel settled successfully"
                );
                persist_settlement_proof(&SettlementProofFields {
                    workload_id,
                    status: &status,
                    unknowns: &settlement_unknowns,
                    elapsed: settlement_elapsed,
                    measured_cpu_ms,
                    settled_at_ms: receipt.settled_at_ms,
                    prepare_ms,
                    execute_ms,
                    queue_wait_ms,
                    queue_wait_clamped,
                    load_tier: &load_tier,
                    capacity_unknowns: &capacity_unknowns,
                });
                receipt
            }
            Err(err) => {
                tracing::error!(%err, workload_id, "lifecycle::finish failed");
                return Err(err);
            }
        };
        if status == AgentCompletionStatus::Completed {
            let proof_refs = evidence_refs
                .iter()
                .map(|reference| format!("{}:{}", reference.kind, reference.digest))
                .collect::<Vec<_>>();
            crate::adapters::agent_execution_runtime_observer::persist_capabilities(
                &["build_completion_and_artifact_read_back"],
                &proof_refs,
            )
            .map_err(|error| format!("build_scheduler_runtime_proof:{error}"))?;
        }
        persist_renewal_proof(workload_id, &guard, renewal_count);
        Ok(())
    }
}

fn persist_renewal_proof(
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

fn persist_cancellation_proof(workload_id: &str, revision: u64) {
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
fn compute_queue_wait_ms(first_claim_ms: u64, committed_at_ms: u64) -> (u64, bool) {
    let raw = first_claim_ms as i64 - committed_at_ms as i64;
    if raw < 0 {
        (0, true)
    } else {
        (raw as u64, false)
    }
}

/// Fields persisted into the settlement proof JSON. Additive across the
/// scheduler-truths tasks: T1 reasons/timings, T2 stage and queue-wait
/// timings, T3 capacity gate outcome.
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
}

/// Build the canonical settlement proof evidence ref. Pure: serializes the
/// settlement facts to canonical JSON and digests them with SHA3-512.
pub(crate) fn settlement_proof_evidence_ref(
    fields: &SettlementProofFields<'_>,
) -> Result<String, serde_json::Error> {
    let proof = serde_json::json!({
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
    });
    cynic_protocol::digest_json(&proof)
        .map(|digest| format!("scheduler-settlement:{digest}"))
}

fn persist_settlement_proof(fields: &SettlementProofFields<'_>) {
    let evidence_ref = settlement_proof_evidence_ref(fields)
        .unwrap_or_else(|_| format!("scheduler-settlement:{}", fields.workload_id));
    if let Err(error) = crate::adapters::agent_execution_runtime_observer::persist_capabilities(
        &["scheduler_settlement"],
        &[evidence_ref],
    ) {
        tracing::warn!(%error, workload_id = %fields.workload_id, "scheduler settlement proof read-back unavailable");
    }
}

fn persist_recovery_proof(workload_id: &str, prior_revision: u64, new_revision: u64) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::agent_workload_store_port::AgentWorkloadStorePort;
    use crate::usecases::build_admission::admit_build_check;
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
        let fields = SettlementProofFields {
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
        };

        let evidence_ref = settlement_proof_evidence_ref(&fields)
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
        let fields = SettlementProofFields {
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
        };

        let evidence_ref =
            settlement_proof_evidence_ref(&fields).expect("settlement proof evidence ref");
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
        let (wait, clamped) = compute_queue_wait_ms(900, 1000);
        assert_eq!(wait, 0, "negative raw wait must clamp to 0");
        assert!(clamped, "clamped flag must be set when skew detected");

        // Normal case: claim after commit
        let (wait, clamped) = compute_queue_wait_ms(1500, 1000);
        assert_eq!(wait, 500, "normal wait must be preserved");
        assert!(!clamped, "clamped flag must be false when no skew");

        // Edge: exact same timestamp
        let (wait, clamped) = compute_queue_wait_ms(1000, 1000);
        assert_eq!(wait, 0, "zero wait must stay 0");
        assert!(!clamped, "zero wait is not skew");
    }

    /// Settlement proof with no unknowns and no measured CPU still produces a
    /// valid evidence ref (None serializes to null in the JSON).
    #[test]
    fn settlement_proof_with_no_unknowns_and_no_cpu() {
        let fields = SettlementProofFields {
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
        };

        let evidence_ref = settlement_proof_evidence_ref(&fields)
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
        });
        let expected_digest = cynic_protocol::digest_json(&expected_proof).expect("digest");
        assert_eq!(digest, expected_digest, "empty settlement proof digest mismatch");
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
            release_publication: None,
        };
        (control, scheduler)
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
}
