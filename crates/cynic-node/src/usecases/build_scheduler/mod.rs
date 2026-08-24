mod evidence;
mod lifecycle;
mod prepare;
mod release_publication;
mod source_prepare;

use std::time::{Duration, Instant};

use cynic_host::adapters::local_evidence_store::LocalContentAddressedEvidenceStore;
use cynic_protocol::{
    AgentBudgets, AgentCitizenshipRecord, AgentCompletionStatus, AgentScope, AgentWorkloadClaim,
    AgentWorkloadState, DurableAgentWorkloadRecord, MutationAdmission, admit_workload_mutation,
    canonical_build_action_from_argv,
};
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

pub(crate) async fn run_loop(control: NodeControlConfig, shutdown: CancellationToken) {
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

async fn run_iteration(
    control: &NodeControlConfig,
    scheduler: &BuildSchedulerConfig,
    store: &LocalAgentWorkloadStore,
    shutdown: &CancellationToken,
) -> Result<(), String> {
    for record in store.active_records().map_err(|error| error.code)? {
        if shutdown.is_cancelled() {
            return Ok(());
        }
        if record.envelope.delegation.node_id != control.node_id
            || canonical_build_action_from_argv(&record.envelope.action.argv).is_none()
        {
            continue;
        }
        let Some(current) = store
            .read(&record.envelope.workload_id)
            .map_err(|error| error.code)?
        else {
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
        execute_claimed(control, scheduler, store, claimed, shutdown).await?;
    }
    Ok(())
}

async fn execute_claimed(
    control: &NodeControlConfig,
    scheduler: &BuildSchedulerConfig,
    store: &LocalAgentWorkloadStore,
    claimed: DurableAgentWorkloadRecord,
    shutdown: &CancellationToken,
) -> Result<(), String> {
    let guard = claimed
        .record
        .claim
        .as_ref()
        .map(|claim| claim.guard.clone())
        .ok_or_else(|| "build_scheduler_claim_missing".to_string())?;

    // Canonical admission gate — verify the citizen has authority to execute
    let citizen = AgentCitizenshipRecord {
        schema: "cynic.agent_citizenship.v1",
        agent_id: claimed.record.envelope.delegation.agent_id.clone(),
        scope: AgentScope::Crate,
        budgets: AgentBudgets {
            actions: 1000,
            wall_seconds: 3600,
        },
        residual_risk: None,
        evidence: None,
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let admission = admit_workload_mutation(
        &citizen,
        Some(&AgentScope::Crate),
        &AgentBudgets {
            actions: 1,
            wall_seconds: 600,
        },
        &AgentScope::Crate,
        true,
        claimed.record.claim.as_ref().map(|c| c.expires_at_ms),
        now_ms,
    );
    if let MutationAdmission::Refused { codes } = &admission {
        return Err(format!("admission_gate_refused:{:?}", codes));
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
            execution_cancel.cancel();
            let renewal_count = monitor.await.unwrap_or(0);
            persist_renewal_proof(&claimed.record.envelope.workload_id, &guard, renewal_count);
            lifecycle::finish_blocked(store, &claimed.record.envelope.workload_id, &guard, error)?;
            return Ok(());
        }
    };
    let execution = TokioBuildExecution
        .execute(&prepared.execution, &execution_cancel)
        .await;
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
    let (status, evidence_refs, unknowns) = match execution {
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
        lifecycle::finish(
            store,
            &latest,
            &guard,
            status,
            evidence_refs.clone(),
            unknowns,
            started.elapsed(),
            measured_cpu_ms,
        )?;
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
