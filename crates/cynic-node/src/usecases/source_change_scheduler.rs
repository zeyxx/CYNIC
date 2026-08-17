use std::time::{Duration, Instant};

use cynic_host::{
    adapters::local_evidence_store::LocalContentAddressedEvidenceStore,
    update_lifecycle::LocalArgvRunner,
};
use cynic_protocol::{
    parse_source_change_submission_input, AgentCompletionStatus, AgentEvidenceRef,
    AgentWorkloadState,
};
use tokio_util::sync::CancellationToken;

use crate::{
    adapters::{
        http_runtime_artifact_transport::HttpRuntimeArtifactTransport,
        kernel_agent_workspace_claim_store::KernelAgentWorkspaceClaimStore,
        local_agent_workload_store::LocalAgentWorkloadStore,
        local_agent_workspace_apply::LocalAgentWorkspaceApplyAdapter,
        local_agent_workspace_lifecycle_store::LocalAgentWorkspaceLifecycleStore,
        local_agent_workspace_observer::LocalAgentWorkspaceObserver,
        local_agent_workspace_provision::LocalAgentWorkspaceProvisioner,
        local_source_blob_store::LocalAgentSourceBlobStore,
        registry::RegistryAgentWorkspaceLayoutAdapter,
    },
    build_scheduler::lifecycle,
    config::{NodeControlConfig, SourceChangeSchedulerConfig},
    ports::{
        agent_workload_store_port::AgentWorkloadStorePort,
        runtime_artifact_transport_port::RuntimeArtifactTransportPort,
        source_change_transaction_port::AgentSourceBlobStorePort,
    },
    source_change_transaction::{
        execute_agent_source_change_transaction, AgentSourceChangeTransactionStatus,
    },
};

pub(crate) async fn run_loop(control: NodeControlConfig, shutdown: CancellationToken) {
    let Some(scheduler) = control.source_change_scheduler.clone() else {
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
            tracing::warn!(%error, "source-change scheduler iteration blocked");
        }
        tokio::select! {
            () = shutdown.cancelled() => break,
            () = tokio::time::sleep(interval) => {}
        }
    }
}

async fn run_iteration(
    control: &NodeControlConfig,
    scheduler: &SourceChangeSchedulerConfig,
    store: &LocalAgentWorkloadStore,
    shutdown: &CancellationToken,
) -> Result<(), String> {
    for record in store.active_records().map_err(|error| error.code)? {
        if shutdown.is_cancelled() || record.envelope.delegation.node_id != control.node_id {
            continue;
        }
        let Ok(submission) = parse_source_change_submission_input(&record.envelope) else {
            continue;
        };
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
                "source-change-scheduler-",
                "cancellation_observed_before_source_change",
            )?;
            continue;
        }
        let Some(claimed) = lifecycle::claim_if_available(
            &control.node_id,
            "source-change-scheduler",
            scheduler.claim_ttl_secs,
            store,
            &current,
        )?
        else {
            continue;
        };
        execute_claimed(control, scheduler, store, claimed, submission, shutdown).await?;
    }
    Ok(())
}

async fn execute_claimed(
    control: &NodeControlConfig,
    scheduler: &SourceChangeSchedulerConfig,
    store: &LocalAgentWorkloadStore,
    claimed: cynic_protocol::DurableAgentWorkloadRecord,
    submission: cynic_protocol::SourceChangeSubmission,
    shutdown: &CancellationToken,
) -> Result<(), String> {
    let workload_id = claimed.record.envelope.workload_id.clone();
    let guard = claimed
        .record
        .claim
        .as_ref()
        .ok_or("source_change_scheduler_claim_missing")?
        .guard
        .clone();
    let started = Instant::now();
    let cancellation = CancellationToken::new();
    let monitor = tokio::spawn(lifecycle::monitor_claim(
        store.clone(),
        workload_id.clone(),
        guard.clone(),
        scheduler.claim_ttl_secs,
        cancellation.clone(),
        shutdown.clone(),
    ));
    let result =
        execute_transaction(control, scheduler, &claimed.record.envelope, &submission).await;
    cancellation.cancel();
    let _ = monitor.await;
    let latest = store
        .read(&workload_id)
        .map_err(|error| error.code)?
        .ok_or("source_change_scheduler_workload_disappeared")?;
    if latest.record.state == AgentWorkloadState::CancelRequested {
        return lifecycle::acknowledge_cancelled(
            store,
            &latest,
            "source-change-scheduler-",
            "source_change_cancelled_during_execution",
        );
    }
    let (status, evidence_refs, unknowns) = match result {
        Ok(result) if result.status == AgentSourceChangeTransactionStatus::Completed => {
            let evidence_refs = result
                .proof_ref
                .into_iter()
                .map(|digest| AgentEvidenceRef {
                    kind: "source_change_read_back".into(),
                    subject: workload_id.clone(),
                    digest,
                    lineage: None,
                })
                .collect::<Vec<_>>();
            let missing = claimed
                .record
                .envelope
                .delegation
                .expected_proofs
                .iter()
                .filter(|proof| {
                    !evidence_refs
                        .iter()
                        .any(|reference| &reference.kind == *proof)
                })
                .cloned()
                .collect::<Vec<_>>();
            if missing.is_empty() {
                (AgentCompletionStatus::Completed, evidence_refs, Vec::new())
            } else {
                (
                    AgentCompletionStatus::Blocked,
                    evidence_refs,
                    missing
                        .into_iter()
                        .map(|proof| format!("source_change_required_proof_missing:{proof}"))
                        .collect(),
                )
            }
        }
        Ok(result) => (
            AgentCompletionStatus::Blocked,
            Vec::new(),
            result.blockers.into_iter().chain(result.unknowns).collect(),
        ),
        Err(error) => (AgentCompletionStatus::Blocked, Vec::new(), vec![error]),
    };
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

async fn execute_transaction(
    control: &NodeControlConfig,
    scheduler: &SourceChangeSchedulerConfig,
    workload: &cynic_protocol::AgentWorkloadEnvelope,
    submission: &cynic_protocol::SourceChangeSubmission,
) -> Result<crate::source_change_transaction::AgentSourceChangeTransactionResult, String> {
    let api_key = std::env::var("CYNIC_KERNEL_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or("source_change_scheduler_kernel_api_key_missing")?;
    let state_root = &control.agent_workload_state_root;
    let blobs = LocalAgentSourceBlobStore::new(state_root.join("source/blobs"));
    let transport = HttpRuntimeArtifactTransport::new(
        state_root.join("source/blobs"),
        control.runtime_artifact_max_bytes,
        Duration::from_secs(control.runtime_artifact_timeout_secs),
    )?;
    for blob in &submission.blobs {
        let destination = blobs
            .stage_destination(&blob.digest)
            .map_err(|error| error.code)?;
        transport
            .stage(&blob.source, &destination, Some(&blob.digest))
            .await?;
        blobs.load(&blob.digest).map_err(|error| error.code)?;
    }
    let layout = RegistryAgentWorkspaceLayoutAdapter::new(&scheduler.registry_path, state_root);
    let runner = LocalArgvRunner;
    let observer = LocalAgentWorkspaceObserver::new(
        &layout,
        &runner,
        "git",
        "id",
        "cynic-node",
        &workload.plan_digest,
    );
    let provisioner = LocalAgentWorkspaceProvisioner::new(&runner, "git", "id");
    let apply = LocalAgentWorkspaceApplyAdapter::new(&blobs);
    let evidence = LocalContentAddressedEvidenceStore::new(state_root.join("evidence"));
    let lifecycle_store =
        LocalAgentWorkspaceLifecycleStore::new(state_root.join("agent-workspaces/lifecycle"));
    let claims = KernelAgentWorkspaceClaimStore::new(
        &control.kernel_url,
        Some(api_key),
        &workload.delegation.agent_id,
    );
    Ok(execute_agent_source_change_transaction(
        workload,
        submission,
        &claims,
        &layout,
        &observer,
        &provisioner,
        &apply,
        &evidence,
        &lifecycle_store,
        lifecycle::current_unix_ms(),
        scheduler.claim_ttl_secs.saturating_mul(1_000),
        scheduler.max_retention_ms,
    )
    .await)
}
