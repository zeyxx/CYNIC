use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cynic_protocol::{
    AGENT_COMPLETION_ENVELOPE_SCHEMA, AgentBudget, AgentCompletionEnvelope, AgentCompletionStatus,
    AgentWorkloadRecord, AgentWorkloadState, DurableAgentWorkloadRecord, LeaseGuard,
    acknowledge_agent_workload_cancellation, claim_agent_workload, complete_agent_workload,
    finalize_unclaimed_agent_workload_cancellation, renew_agent_workload_claim,
};
use tokio_util::sync::CancellationToken;

use crate::{
    adapters::local_agent_workload_store::LocalAgentWorkloadStore,
    config::{BuildSchedulerConfig, NodeControlConfig},
    ports::agent_workload_store_port::AgentWorkloadStorePort,
};

pub(super) fn claim_if_available(
    control: &NodeControlConfig,
    scheduler: &BuildSchedulerConfig,
    store: &LocalAgentWorkloadStore,
    current: &DurableAgentWorkloadRecord,
) -> Result<Option<DurableAgentWorkloadRecord>, String> {
    let now_ms = current_unix_ms();
    if current.record.state == AgentWorkloadState::Claimed
        && current
            .record
            .claim
            .as_ref()
            .is_some_and(|claim| claim.expires_at_ms > now_ms)
    {
        return Ok(None);
    }
    let guard = LeaseGuard {
        agent_id: current.record.envelope.delegation.agent_id.clone(),
        target: current.record.envelope.workload_id.clone(),
        lease_id: format!(
            "build-scheduler-{}-{}-{}",
            control.node_id,
            now_ms,
            current.record.last_fencing_token.saturating_add(1)
        ),
        fencing_token: current.record.last_fencing_token.saturating_add(1),
    };
    let lease_ms = scheduler.claim_ttl_secs.saturating_mul(1_000);
    let claimed = claim_agent_workload(
        &current.record,
        guard,
        now_ms.saturating_add(lease_ms),
        now_ms,
        lease_ms,
    )
    .map_err(|error| error.code)?;
    match store.compare_and_swap(
        &current.record.envelope.workload_id,
        Some(current.revision),
        &claimed,
        now_ms,
    ) {
        Ok(durable) => Ok(Some(durable)),
        Err(error) if error.code == "workload_store_revision_conflict" => Ok(None),
        Err(error) => Err(error.code),
    }
}

pub(super) async fn monitor_claim(
    store: LocalAgentWorkloadStore,
    workload_id: String,
    guard: LeaseGuard,
    claim_ttl_secs: u64,
    cancellation: CancellationToken,
    shutdown: CancellationToken,
) -> u64 {
    let mut successful_renewals = 0;
    loop {
        tokio::select! {
            () = shutdown.cancelled() => {
                cancellation.cancel();
                break successful_renewals;
            }
            () = cancellation.cancelled() => break successful_renewals,
            () = tokio::time::sleep(Duration::from_secs(1)) => {
                let durable = match store.read(&workload_id) {
                    Ok(Some(durable)) => durable,
                    _ => {
                        cancellation.cancel();
                        break successful_renewals;
                    }
                };
                if durable.record.state == AgentWorkloadState::CancelRequested
                    || durable.record.claim.as_ref().map(|claim| &claim.guard) != Some(&guard)
                {
                    cancellation.cancel();
                    break successful_renewals;
                }
                let now_ms = current_unix_ms();
                let ttl_ms = claim_ttl_secs.saturating_mul(1_000);
                let renewal_due = durable
                    .record
                    .claim
                    .as_ref()
                    .is_some_and(|claim| claim.expires_at_ms <= now_ms.saturating_add(ttl_ms / 2));
                if renewal_due {
                    let renewed = match renew_agent_workload_claim(
                        &durable.record,
                        &guard,
                        now_ms.saturating_add(ttl_ms),
                        now_ms,
                    ) {
                        Ok(renewed) => renewed,
                        Err(_) => {
                            cancellation.cancel();
                            break successful_renewals;
                        }
                    };
                    match store.compare_and_swap(
                        &workload_id,
                        Some(durable.revision),
                        &renewed,
                        now_ms,
                    ) {
                        Ok(_) => successful_renewals += 1,
                        Err(error) if error.code == "workload_store_revision_conflict" => {}
                        Err(_) => {
                            cancellation.cancel();
                            break successful_renewals;
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn finish_blocked(
    store: &LocalAgentWorkloadStore,
    workload_id: &str,
    guard: &LeaseGuard,
    unknown: String,
) -> Result<(), String> {
    let latest = store
        .read(workload_id)
        .map_err(|error| error.code)?
        .ok_or_else(|| "build_scheduler_workload_disappeared".to_string())?;
    if latest.record.state == AgentWorkloadState::CancelRequested {
        return acknowledge_cancelled(store, &latest, &unknown);
    }
    finish(
        store,
        &latest,
        guard,
        AgentCompletionStatus::Blocked,
        Vec::new(),
        vec![unknown],
        Duration::ZERO,
    )
}

pub(super) fn acknowledge_cancelled(
    store: &LocalAgentWorkloadStore,
    durable: &DurableAgentWorkloadRecord,
    reason: &str,
) -> Result<(), String> {
    let completion = completion(
        &durable.record,
        AgentCompletionStatus::Blocked,
        Vec::new(),
        vec![reason.to_string()],
        Duration::ZERO,
    );
    let next = match durable.record.claim.as_ref() {
        Some(claim) if claim.guard.lease_id.starts_with("build-scheduler-") => {
            acknowledge_agent_workload_cancellation(&durable.record, &claim.guard, &completion)
                .map_err(|error| error.code)?
        }
        Some(_) => return Ok(()),
        None => finalize_unclaimed_agent_workload_cancellation(&durable.record, &completion)
            .map_err(|error| error.code)?,
    };
    store
        .compare_and_swap(
            &durable.record.envelope.workload_id,
            Some(durable.revision),
            &next,
            current_unix_ms(),
        )
        .map(|_| ())
        .map_err(|error| error.code)
}

pub(super) fn finish(
    store: &LocalAgentWorkloadStore,
    durable: &DurableAgentWorkloadRecord,
    guard: &LeaseGuard,
    status: AgentCompletionStatus,
    evidence_refs: Vec<cynic_protocol::AgentEvidenceRef>,
    unknowns: Vec<String>,
    elapsed: Duration,
) -> Result<(), String> {
    let completion = completion(&durable.record, status, evidence_refs, unknowns, elapsed);
    let next = complete_agent_workload(&durable.record, guard, &completion, current_unix_ms())
        .map_err(|error| error.code)?;
    store
        .compare_and_swap(
            &durable.record.envelope.workload_id,
            Some(durable.revision),
            &next,
            current_unix_ms(),
        )
        .map(|_| ())
        .map_err(|error| error.code)
}

fn completion(
    record: &AgentWorkloadRecord,
    status: AgentCompletionStatus,
    evidence_refs: Vec<cynic_protocol::AgentEvidenceRef>,
    unknowns: Vec<String>,
    elapsed: Duration,
) -> AgentCompletionEnvelope {
    AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.into(),
        delegation_id: record.envelope.delegation.delegation_id.clone(),
        node_id: record.envelope.delegation.node_id.clone(),
        agent_id: record.envelope.delegation.agent_id.clone(),
        parent_agent_id: record.envelope.delegation.parent_agent_id.clone(),
        status,
        evidence_refs,
        unknowns,
        consumed_budget: AgentBudget {
            context_tokens: 0,
            actions: record.consumed_actions,
            children: 0,
            wall_time_ms: elapsed.as_millis().min(u128::from(u64::MAX)) as u64,
        },
    }
}

pub(super) fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}
