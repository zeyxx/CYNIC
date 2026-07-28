use std::{
    fs,
    path::PathBuf,
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use cynic_protocol::{
    AGENT_SCHEDULER_JOURNAL_RECORD_SCHEMA, AgentSchedulerJournalRecord,
    AgentSchedulerJournalStatus, AgentSchedulerStepObservation, digest_json,
    reconcile_agent_scheduler_step,
};

use crate::{
    adapters::local_agent_scheduler_journal::LocalAgentSchedulerJournal,
    ports::agent_scheduler_port::AgentSchedulerJournalPort,
};

pub fn execute_agent_scheduler_step(observation_path: &str, state_root: &str) -> ExitCode {
    let bytes = match fs::read(observation_path) {
        Ok(bytes) => bytes,
        Err(failure) => return emit_error(&format!("scheduler_observation_read:{failure}")),
    };
    let observation: AgentSchedulerStepObservation = match serde_json::from_slice(&bytes) {
        Ok(observation) => observation,
        Err(failure) => return emit_error(&format!("scheduler_observation_invalid:{failure}")),
    };
    let plan = reconcile_agent_scheduler_step(&observation);
    let plan_digest = match digest_json(&plan) {
        Ok(digest) => digest,
        Err(failure) => return emit_error(&format!("scheduler_plan_digest:{failure}")),
    };
    let record = AgentSchedulerJournalRecord {
        schema: AGENT_SCHEDULER_JOURNAL_RECORD_SCHEMA.to_string(),
        node_id: plan.node_id.clone(),
        scheduler_epoch: plan.scheduler_epoch,
        step_id: plan.step_id.clone(),
        plan_digest,
        plan: Some(plan.clone()),
        status: AgentSchedulerJournalStatus::Planned,
        evidence_refs: Vec::new(),
        unknowns: Vec::new(),
        recorded_at_ms: current_unix_ms(),
    };
    let journal =
        LocalAgentSchedulerJournal::new(PathBuf::from(state_root).join("agent-scheduler/journal"));
    let persisted = match journal.persist(&record) {
        Ok(record) => record,
        Err(failure) => return emit_error(&failure.code),
    };
    emit(
        &serde_json::json!({
            "schema": "cynic.node.agent_scheduler_step_report.v1",
            "status": "planned",
            "mutates_runtime": false,
            "plan": plan,
            "journal": persisted,
            "activation": "inactive"
        }),
        true,
    )
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn emit_error(blocker: &str) -> ExitCode {
    emit(
        &serde_json::json!({
            "schema": "cynic.node.agent_scheduler_step_report.v1",
            "status": "blocked",
            "mutates_runtime": false,
            "blockers": [blocker],
            "activation": "inactive"
        }),
        false,
    )
}

fn emit(value: &impl serde::Serialize, success: bool) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(failure) => {
            eprintln!("scheduler step serialization failed: {failure}");
            return ExitCode::FAILURE;
        }
    }
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
