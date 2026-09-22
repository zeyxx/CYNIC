use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use cynic_host::adapters::local_evidence_store::LocalContentAddressedEvidenceStore;
use cynic_lifecycle::LocalArgvRunner;

use crate::{
    adapters::{
        kernel_agent_workspace_claim_store::KernelAgentWorkspaceClaimStore,
        local_agent_workspace_apply::LocalAgentWorkspaceApplyAdapter,
        local_agent_workspace_lifecycle_store::LocalAgentWorkspaceLifecycleStore,
        local_agent_workspace_observer::LocalAgentWorkspaceObserver,
        local_agent_workspace_provision::LocalAgentWorkspaceProvisioner,
        local_source_blob_store::LocalAgentSourceBlobStore,
        registry::RegistryAgentWorkspaceLayoutAdapter,
    },
    ports::source_change_transaction_port::AgentSourceBlobStorePort,
    source_change_transaction::{
        AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA, AgentSourceChangeTransactionRequest,
        AgentSourceChangeTransactionResult, AgentSourceChangeTransactionStatus,
        execute_agent_source_change_transaction,
    },
};

pub async fn execute_agent_source_change(
    request_path: &str,
    result_file: Option<&str>,
    registry_path: &str,
    state_root: &str,
    kernel_endpoint: &str,
    max_claim_ttl_ms: u64,
    max_retention_ms: u64,
    apply: bool,
) -> ExitCode {
    let request = match read_request(request_path) {
        Ok(request) => request,
        Err(error) => return emit_error(error),
    };
    if !apply {
        return emit_value(
            &serde_json::json!({
                "schema": "cynic.node.agent_source_change_plan.v1",
                "status": "planned",
                "mutates": true,
                "requires_principal": "cynic",
                "request": request,
                "next": "repeat with --apply on the declared node"
            }),
            true,
        );
    }

    let kernel_api_key =
        match cynic_secrets::env_var("kernel-api-key-agent-src", "CYNIC_KERNEL_API_KEY")
            .filter(|value| !value.is_empty())
        {
            Some(api_key) => api_key,
            None => {
                return emit_error(
                "kernel_api_key_missing:CYNIC_KERNEL_API_KEY is required for agent-bound claims"
                    .to_string(),
            );
            }
        };
    let state_root = PathBuf::from(state_root);
    let blobs = LocalAgentSourceBlobStore::new(state_root.join("source/blobs"));
    if let Err(error) = persist_result_blob(&request, result_file, &blobs) {
        return emit_error(error);
    }
    let evidence = LocalContentAddressedEvidenceStore::new(state_root.join("evidence"));
    let lifecycle =
        LocalAgentWorkspaceLifecycleStore::new(state_root.join("agent-workspaces/lifecycle"));
    let layout = RegistryAgentWorkspaceLayoutAdapter::new(registry_path, &state_root);
    let runner = LocalArgvRunner;
    let observer = LocalAgentWorkspaceObserver::new(
        &layout,
        &runner,
        "git",
        "id",
        "cynic-node",
        &request.workload.plan_digest,
    );
    let provisioner = LocalAgentWorkspaceProvisioner::new(&runner, "git");
    let apply_adapter = LocalAgentWorkspaceApplyAdapter::new(&blobs);
    let claim_store = KernelAgentWorkspaceClaimStore::new(
        kernel_endpoint,
        Some(kernel_api_key),
        &request.workload.delegation.agent_id,
    );
    let mut result: AgentSourceChangeTransactionResult = execute_agent_source_change_transaction(
        &request,
        &claim_store,
        &layout,
        &observer,
        &provisioner,
        &apply_adapter,
        &evidence,
        &lifecycle,
        current_unix_ms(),
        max_claim_ttl_ms,
        max_retention_ms,
    )
    .await;
    let mut success = result.schema == AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA
        && result.status == AgentSourceChangeTransactionStatus::Completed;
    if success {
        let evidence_refs = result.proof_ref.clone().into_iter().collect::<Vec<_>>();
        let evidence_refs = if evidence_refs.is_empty() {
            cynic_protocol::digest_json(&result)
                .map(|digest| vec![format!("source-change-transaction:{digest}")])
                .unwrap_or_default()
        } else {
            evidence_refs
        };
        if let Err(error) = crate::adapters::agent_execution_runtime_observer::persist_capabilities(
            &[
                "node_workspace_observer_wiring_and_proof",
                "workspace_claim_store_wiring_and_conformance_proof",
                "bounded_source_change_apply_adapter",
                "post_apply_source_read_back",
            ],
            &evidence_refs,
        ) {
            result
                .unknowns
                .push(format!("source_change_runtime_proof:{error}"));
            success = false;
        }
    }
    emit_value(&result, success)
}

fn read_request(path: &str) -> Result<AgentSourceChangeTransactionRequest, String> {
    let bytes = fs::read(path).map_err(|error| format!("source_change_request_read:{error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("source_change_request_invalid:{error}"))
}

fn persist_result_blob(
    request: &AgentSourceChangeTransactionRequest,
    result_file: Option<&str>,
    store: &LocalAgentSourceBlobStore,
) -> Result<(), String> {
    if request.changes.len() != 1 {
        return Err("source_change_first_vertical_requires_one_path".to_string());
    }
    match (
        request.changes[0].result_digest.as_deref(),
        result_file.map(Path::new),
    ) {
        (Some(digest), Some(path)) => {
            let content = fs::read(path).map_err(|error| format!("source_result_read:{error}"))?;
            store
                .persist(digest, &content)
                .map_err(|error| format!("source_result_blob:{}:{}", error.code, error.message))
        }
        (Some(_), None) => Err("source_result_file_missing".to_string()),
        (None, Some(_)) => Err("source_result_file_unexpected_for_delete".to_string()),
        (None, None) => Ok(()),
    }
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn emit_error(error: String) -> ExitCode {
    emit_value(
        &serde_json::json!({
            "schema": "cynic.node.agent_source_change_error.v1",
            "status": "blocked",
            "blockers": [error]
        }),
        false,
    )
}

fn emit_value(value: &impl serde::Serialize, success: bool) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("agent source change report serialization failed: {error}");
            return ExitCode::FAILURE;
        }
    }
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
