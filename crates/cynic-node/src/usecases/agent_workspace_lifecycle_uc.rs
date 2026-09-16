use std::{
    path::PathBuf,
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use cynic_lifecycle::LocalArgvRunner;
use cynic_workload::{
    AgentWorkspaceLifecycleDecision,
    reconcile_agent_workspace_lifecycle,
};

use crate::{
    adapters::{
        kernel_agent_workspace_claim_store::KernelAgentWorkspaceClaimStore,
        local_agent_workspace_collection::LocalAgentWorkspaceCollectionAdapter,
        local_agent_workspace_lifecycle_observer::LocalAgentWorkspaceLifecycleObserver,
        local_agent_workspace_lifecycle_store::LocalAgentWorkspaceLifecycleStore,
        registry::RegistryAgentWorkspaceLayoutAdapter,
    },
    ports::agent_workspace_port::{
        AgentWorkspaceClaimStorePort, AgentWorkspaceCollectionPort, AgentWorkspaceLayoutPort,
        AgentWorkspaceLifecycleStorePort, AgentWorkspaceTerminalObservationPort,
    },
};

#[allow(clippy::too_many_arguments)]
pub async fn execute_agent_workspace_lifecycle(
    workload_id: &str,
    registry_path: &str,
    state_root: &str,
    kernel_endpoint: &str,
    agent_id: &str,
    renewal_window_ms: u64,
    renewal_ttl_ms: u64,
    apply: bool,
) -> ExitCode {
    let state_root = PathBuf::from(state_root);
    let store =
        LocalAgentWorkspaceLifecycleStore::new(state_root.join("agent-workspaces/lifecycle"));
    let record = match store.load(workload_id) {
        Ok(Some(record)) => record,
        Ok(None) => return emit_unknown("workspace_lifecycle_record_missing"),
        Err(failure) => return emit_unknown(&failure.code),
    };
    let layout_adapter = RegistryAgentWorkspaceLayoutAdapter::new(registry_path, &state_root);
    let layout = match layout_adapter.resolve(
        &record.repository_id,
        &record.workspace_id,
        &record.node_id,
    ) {
        Ok(layout) => layout,
        Err(failure) => return emit_unknown(&failure.code),
    };
    let api_key = match std::env::var("CYNIC_KERNEL_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
    {
        Some(value) => value,
        None => return emit_unknown("kernel_api_key_missing"),
    };
    let claims = KernelAgentWorkspaceClaimStore::new(kernel_endpoint, Some(api_key), agent_id);
    let now_ms = current_unix_ms();
    let repository_claims = match claims
        .repository_claims(&record.repository_id, now_ms)
        .await
    {
        Ok(claims) => claims,
        Err(failure) => return emit_unknown(&failure.code),
    };
    let runner = LocalArgvRunner;
    let observer = LocalAgentWorkspaceLifecycleObserver::new(&runner, "git");
    let observation = match observer.observe_terminal(&layout, &record, repository_claims) {
        Ok(observation) => observation,
        Err(failure) => return emit_unknown(&failure.code),
    };
    let plan = reconcile_agent_workspace_lifecycle(
        &observation,
        now_ms,
        renewal_window_ms,
        renewal_ttl_ms,
    );
    if !apply {
        return emit(
            &serde_json::json!({
                "schema": "cynic.node.agent_workspace_lifecycle_report.v1",
                "status": "planned",
                "mutates": plan.mutates,
                "observation": observation,
                "plan": plan
            }),
            true,
        );
    }
    if !matches!(
        &plan.decision,
        AgentWorkspaceLifecycleDecision::Collect { .. }
    ) {
        return emit(
            &serde_json::json!({
                "schema": "cynic.node.agent_workspace_lifecycle_report.v1",
                "status": "blocked",
                "mutates": false,
                "plan": plan,
                "blockers": ["only_collect_is_executable_on_this_surface"]
            }),
            false,
        );
    }
    let collector = LocalAgentWorkspaceCollectionAdapter::new(&runner, "git", "df");
    match collector.collect(&layout, &plan) {
        Ok(read_back) => emit(
            &serde_json::json!({
                "schema": "cynic.node.agent_workspace_lifecycle_report.v1",
                "status": "collected",
                "mutates": true,
                "plan": plan,
                "read_back": read_back
            }),
            true,
        ),
        Err(failure) => emit(
            &serde_json::json!({
                "schema": "cynic.node.agent_workspace_lifecycle_report.v1",
                "status": "blocked",
                "mutates": true,
                "plan": plan,
                "blockers": [failure.code],
                "detail": failure.message
            }),
            false,
        ),
    }
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn emit_unknown(blocker: &str) -> ExitCode {
    emit(
        &serde_json::json!({
            "schema": "cynic.node.agent_workspace_lifecycle_report.v1",
            "status": "unknown",
            "mutates": false,
            "unknowns": [blocker]
        }),
        false,
    )
}

fn emit(value: &impl serde::Serialize, success: bool) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(failure) => {
            eprintln!("workspace lifecycle serialization failed: {failure}");
            return ExitCode::FAILURE;
        }
    }
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
