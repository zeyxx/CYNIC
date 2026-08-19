use std::{
    fs,
    path::PathBuf,
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use cynic_protocol::{
    AGENT_WORKLOAD_RECORD_SCHEMA, AGENT_WORKLOAD_TRANSITION_REQUEST_SCHEMA, AgentWorkloadDecision,
    AgentWorkloadRecord, AgentWorkloadState, AgentWorkloadTransition,
    AgentWorkloadTransitionRequest, acknowledge_agent_workload_cancellation,
    admit_agent_workload_dispatch, admit_aggregate_sibling_budget, claim_agent_workload,
    complete_agent_workload, consume_agent_workload_action,
    finalize_unclaimed_agent_workload_cancellation, renew_agent_workload_claim,
    request_agent_workload_cancellation,
};

use crate::{
    adapters::local_agent_workload_store::LocalAgentWorkloadStore,
    ports::agent_workload_store_port::AgentWorkloadStorePort,
};

pub fn execute_agent_workload_status(workload_id: &str, state_root: &str) -> ExitCode {
    let store = store(state_root);
    match store.read(workload_id) {
        Ok(record) => emit(
            &serde_json::json!({
                "schema": "cynic.node.agent_workload_status.v1",
                "status": if record.is_some() { "observed" } else { "unknown" },
                "mutates_runtime": false,
                "workload_id": workload_id,
                "durable_record": record,
            }),
            true,
        ),
        Err(failure) => emit_error(&failure.code, &failure.detail),
    }
}

pub fn execute_agent_workload_transition(
    request_path: &str,
    state_root: &str,
    apply: bool,
) -> ExitCode {
    let bytes = match fs::read(request_path) {
        Ok(bytes) => bytes,
        Err(failure) => {
            return emit_error(
                "workload_transition_request_read_failed",
                &failure.to_string(),
            );
        }
    };
    let request: AgentWorkloadTransitionRequest = match serde_json::from_slice(&bytes) {
        Ok(request) => request,
        Err(failure) => {
            return emit_error("workload_transition_request_invalid", &failure.to_string());
        }
    };
    if request.schema != AGENT_WORKLOAD_TRANSITION_REQUEST_SCHEMA {
        return emit_error(
            "workload_transition_request_rejected",
            "request schema is unsupported",
        );
    }
    let observed_now_ms = current_unix_ms();

    let workload_id = request.transition.workload_id().to_string();
    let store = store(state_root);
    let observed = match store.read(&workload_id) {
        Ok(record) => record,
        Err(failure) => return emit_error(&failure.code, &failure.detail),
    };
    let next = match reconcile_transition(&store, &request, observed.as_ref(), observed_now_ms) {
        Ok(record) => record,
        Err((code, detail)) => return emit_error(&code, &detail),
    };

    if !apply {
        return emit(
            &serde_json::json!({
                "schema": "cynic.node.agent_workload_transition_report.v1",
                "status": "planned",
                "mutates_runtime": false,
                "workload_id": workload_id,
                "expected_revision": request.expected_revision,
                "next_record": next,
                "apply_required": true,
            }),
            true,
        );
    }

    match store.compare_and_swap(
        &workload_id,
        request.expected_revision,
        &next,
        observed_now_ms,
    ) {
        Ok(durable) => emit(
            &serde_json::json!({
                "schema": "cynic.node.agent_workload_transition_report.v1",
                "status": "applied",
                "mutates_runtime": true,
                "workload_id": workload_id,
                "read_back": durable,
            }),
            true,
        ),
        Err(failure) => emit_error(&failure.code, &failure.detail),
    }
}

fn reconcile_transition(
    store: &dyn AgentWorkloadStorePort,
    request: &AgentWorkloadTransitionRequest,
    observed: Option<&cynic_protocol::DurableAgentWorkloadRecord>,
    observed_now_ms: u64,
) -> Result<AgentWorkloadRecord, (String, String)> {
    match &request.transition {
        AgentWorkloadTransition::Dispatch { envelope } => {
            if request.expected_revision.is_some() || observed.is_some() {
                return Err(error(
                    "workload_dispatch_already_exists",
                    "dispatch requires absent workload state and no expected revision",
                ));
            }
            let active = store
                .active_idempotency_keys()
                .map_err(|failure| (failure.code, failure.detail))?;
            let admission = admit_agent_workload_dispatch(envelope, &active);
            if !admission.admitted() {
                let detail = match admission.decision {
                    AgentWorkloadDecision::Deny { violations } => violations
                        .into_iter()
                        .map(|violation| violation.code)
                        .collect::<Vec<_>>()
                        .join(","),
                    AgentWorkloadDecision::Unknown { missing_evidence } => {
                        missing_evidence.join(",")
                    }
                    AgentWorkloadDecision::Allow => "admission_not_confirmed".to_string(),
                };
                return Err(error("workload_dispatch_not_admitted", &detail));
            }
            let active_records = store
                .active_records()
                .map_err(|failure| (failure.code, failure.detail))?;
            admit_aggregate_sibling_budget(envelope, &active_records)
                .map_err(|code| error(&code, &code))?;
            Ok(AgentWorkloadRecord {
                schema: AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
                envelope: envelope.clone(),
                state: AgentWorkloadState::Dispatched,
                claim: None,
                last_fencing_token: 0,
                cancellation_generation: 0,
                consumed_actions: 0,
                completion: None,
            })
        }
        transition => {
            let current = observed.ok_or_else(|| {
                error(
                    "workload_transition_state_missing",
                    "non-dispatch transition requires durable observed state",
                )
            })?;
            if request.expected_revision != Some(current.revision) {
                return Err(error(
                    "workload_transition_revision_mismatch",
                    "expected revision must match the durable read-back",
                ));
            }
            let result = match transition {
                AgentWorkloadTransition::Claim {
                    guard,
                    expires_at_ms,
                    ..
                } => claim_agent_workload(
                    &current.record,
                    guard.clone(),
                    *expires_at_ms,
                    observed_now_ms,
                ),
                AgentWorkloadTransition::ConsumeAction { guard, .. } => {
                    consume_agent_workload_action(&current.record, guard, observed_now_ms)
                }
                AgentWorkloadTransition::RenewClaim {
                    guard,
                    expires_at_ms,
                    ..
                } => renew_agent_workload_claim(
                    &current.record,
                    guard,
                    *expires_at_ms,
                    observed_now_ms,
                ),
                AgentWorkloadTransition::RequestCancellation { .. } => {
                    request_agent_workload_cancellation(&current.record)
                }
                AgentWorkloadTransition::AcknowledgeCancellation {
                    guard, completion, ..
                } => acknowledge_agent_workload_cancellation(&current.record, guard, completion),
                AgentWorkloadTransition::FinalizeUnclaimedCancellation { completion, .. } => {
                    finalize_unclaimed_agent_workload_cancellation(&current.record, completion)
                }
                AgentWorkloadTransition::Complete {
                    guard, completion, ..
                } => complete_agent_workload(&current.record, guard, completion, observed_now_ms),
                AgentWorkloadTransition::Dispatch { .. } => unreachable!(),
            };
            result.map_err(|violation| (violation.code, violation.message))
        }
    }
}

fn store(state_root: &str) -> LocalAgentWorkloadStore {
    LocalAgentWorkloadStore::new(PathBuf::from(state_root).join("agent-scheduler/workloads"))
}

pub fn execute_agent_workload_list(state_root: &str) -> ExitCode {
    let store = store(state_root);
    match store.durable_records() {
        Ok(records) => emit(
            &serde_json::json!({
                "schema": "cynic.node.agent_workload_list.v1",
                "status": "observed",
                "mutates_runtime": false,
                "count": records.len(),
                "workloads": records,
            }),
            true,
        ),
        Err(failure) => emit_error(&failure.code, &failure.detail),
    }
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn error(code: &str, detail: &str) -> (String, String) {
    (code.to_string(), detail.to_string())
}

fn emit_error(code: &str, detail: &str) -> ExitCode {
    emit(
        &serde_json::json!({
            "schema": "cynic.node.agent_workload_transition_report.v1",
            "status": "blocked",
            "mutates_runtime": false,
            "blockers": [{"code": code, "detail": detail}],
        }),
        false,
    )
}

fn emit(value: &impl serde::Serialize, success: bool) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(failure) => {
            eprintln!("agent workload report serialization failed: {failure}");
            return ExitCode::FAILURE;
        }
    }
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
