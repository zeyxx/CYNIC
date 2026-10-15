//! Claimed-workload execution: guard, prepare, execute, settle.
//!
//! One orchestration function over the extracted stages (`prepare`,
//! `lifecycle`, `evidence`, `release_publication`) and the footprint
//! measures (`measure`). No policy lives here: admission decided upstream,
//! settlement facts flow into `measure` for proof persistence.

use std::time::Instant;

use cynic_host::adapters::local_evidence_store::LocalContentAddressedEvidenceStore;
use cynic_protocol::AgentCompletionStatus;
use cynic_workload::{AgentWorkloadState, DurableAgentWorkloadRecord, admit_agent_workload};
use tokio_util::sync::CancellationToken;

use super::measure::{
    SettlementProofFields, compute_queue_wait_ms, effective_target_dir, measure_max_rss_kb,
    measure_target_dir_size_bytes, persist_cancellation_proof, persist_renewal_proof,
    persist_settlement_proof,
};
use super::{evidence, lifecycle, prepare, release_publication};
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

pub(crate) async fn execute_claimed(
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
    // Footprint measurement scope: target-dir growth (du-style) brackets the
    // build on the effective CARGO_TARGET_DIR. RSS is measured after the build
    // via getrusage(RUSAGE_CHILDREN) — no "before" needed.
    // NOTE: target-dir growth is approximate under concurrency because multiple
    // builds may share the same target dir.
    let target_dir = effective_target_dir(
        scheduler.shared_target_root.as_deref(),
        &prepared.workspace.workspace_root,
        &claimed.record.envelope.action.argv,
    );
    let target_dir_size_before = measure_target_dir_size_bytes(&target_dir);
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
    // Close the footprint measurement scope: RSS via RUSAGE_CHILDREN
    // (max of the build child tree, not the scheduler's own RSS) and
    // target-dir growth (du-style: size of effective target dir before/after
    // the execute window). Approximate under concurrency (shared target).
    // None = unmeasured (explicit unknown).
    let max_rss_kb = measure_max_rss_kb();
    let disk_delta_bytes = match (target_dir_size_before, measure_target_dir_size_bytes(&target_dir)) {
        (Some(before), Some(after)) => Some(after as i64 - before as i64),
        _ => None,
    };
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
                persist_settlement_proof(store, &SettlementProofFields {
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
                    load_tier,
                    capacity_unknowns,
                    max_rss_kb,
                    disk_delta_bytes,
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
