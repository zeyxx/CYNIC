mod evidence;
mod lifecycle;
mod prepare;
mod release_publication;
mod source_prepare;

use std::time::{Duration, Instant};

use cynic_host::adapters::local_evidence_store::LocalContentAddressedEvidenceStore;
use cynic_protocol::{
    AgentCompletionStatus, AgentWorkloadState, DurableAgentWorkloadRecord,
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
            continue;
        }
        let Some(claimed) = lifecycle::claim_if_available(control, scheduler, store, &current)?
        else {
            continue;
        };
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
    let started = Instant::now();
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
    ) {
        Ok(prepared) => prepared,
        Err(error) => {
            execution_cancel.cancel();
            let _ = monitor.await;
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
        let _ = monitor.await;
        lifecycle::acknowledge_cancelled(
            store,
            &after_execution,
            "workload_cancelled_during_execution",
        )?;
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
                        (AgentCompletionStatus::Blocked, evidence_refs, vec![error])
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
                            AgentCompletionStatus::Blocked,
                            evidence_refs,
                            vec!["build_scheduler_required_proof_missing".to_string()],
                        )
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
    let _ = monitor.await;
    let latest = store
        .read(workload_id)
        .map_err(|error| error.code)?
        .ok_or_else(|| "build_scheduler_workload_disappeared".to_string())?;
    if latest.record.state == AgentWorkloadState::CancelRequested {
        lifecycle::acknowledge_cancelled(store, &latest, "workload_cancelled_during_proof")
    } else {
        lifecycle::finish(
            store,
            &latest,
            &guard,
            status,
            evidence_refs,
            unknowns,
            started.elapsed(),
        )
    }
}
