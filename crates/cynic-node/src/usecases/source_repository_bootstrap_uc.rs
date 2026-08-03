use std::{
    path::PathBuf,
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use cynic_host::update_lifecycle::LocalArgvRunner;
use cynic_protocol::{
    SOURCE_REPOSITORY_BOOTSTRAP_REPORT_SCHEMA, SOURCE_REPOSITORY_BOOTSTRAP_REQUEST_SCHEMA,
    SourceRepositoryBootstrapDecision, SourceRepositoryBootstrapProofAuthority,
    SourceRepositoryBootstrapReport, SourceRepositoryBootstrapRequest,
    SourceRepositoryBootstrapStatus, reconcile_source_repository_bootstrap,
};

use crate::{
    adapters::{
        kernel_source_repository_proof_commit::KernelSourceRepositoryProofCommit,
        local_source_repository_bootstrap::LocalSourceRepositoryBootstrap,
        registry::RegistryAgentWorkspaceLayoutAdapter,
    },
    domain::source_repository_proof::{
        mark_kernel_committed, mark_unknown, persist_local_proof, runtime_proof_bundle,
    },
    ports::{
        agent_workspace_port::AgentWorkspaceLayoutPort,
        source_repository_bootstrap_port::SourceRepositoryBootstrapPort,
        source_repository_proof_commit_port::SourceRepositoryProofCommitPort,
    },
};

pub async fn execute_source_repository_bootstrap(
    node_id: &str,
    repository_id: &str,
    source: &str,
    revision: &str,
    registry_path: &str,
    state_root: &str,
    agent_id: &str,
    kernel_endpoint: &str,
    apply: bool,
) -> ExitCode {
    let request = SourceRepositoryBootstrapRequest {
        schema: SOURCE_REPOSITORY_BOOTSTRAP_REQUEST_SCHEMA.to_string(),
        node_id: node_id.to_string(),
        repository_id: repository_id.to_string(),
        source: source.to_string(),
        revision: revision.to_string(),
    };
    let state_root_path = PathBuf::from(state_root);
    let layout_port = RegistryAgentWorkspaceLayoutAdapter::new(registry_path, &state_root_path);
    let layout = match layout_port.resolve(repository_id, "source-bootstrap", node_id) {
        Ok(layout) => layout,
        Err(failure) => {
            return emit_error(format!(
                "source_bootstrap_layout:{}:{}",
                failure.code, failure.message
            ));
        }
    };
    let runner = LocalArgvRunner;
    let adapter = LocalSourceRepositoryBootstrap::new(&runner, &state_root_path, "git", "id");
    let before = match adapter.observe(&layout, &request) {
        Ok(observation) => observation,
        Err(failure) => {
            return emit_error(format!(
                "source_bootstrap_observation:{}:{}",
                failure.code, failure.message
            ));
        }
    };
    let decision = reconcile_source_repository_bootstrap(&request, &before);
    let repository_path = layout.repository_root.to_string_lossy().into_owned();
    let planned_actions = vec![
        vec![
            "git".to_string(),
            "clone".to_string(),
            "--no-checkout".to_string(),
            "--origin".to_string(),
            "origin".to_string(),
            source.to_string(),
            repository_path.clone(),
        ],
        vec![
            "git".to_string(),
            "-C".to_string(),
            repository_path,
            "checkout".to_string(),
            "--detach".to_string(),
            revision.to_string(),
        ],
    ];

    if !apply {
        let status = match &decision {
            SourceRepositoryBootstrapDecision::Converged => {
                SourceRepositoryBootstrapStatus::Completed
            }
            SourceRepositoryBootstrapDecision::BootstrapRequired => {
                SourceRepositoryBootstrapStatus::Planned
            }
            SourceRepositoryBootstrapDecision::SyncRequired => {
                SourceRepositoryBootstrapStatus::Planned
            }
            SourceRepositoryBootstrapDecision::Blocked { .. } => {
                SourceRepositoryBootstrapStatus::Blocked
            }
            SourceRepositoryBootstrapDecision::Unknown { .. } => {
                SourceRepositoryBootstrapStatus::Unknown
            }
        };
        return emit_report(report(
            status,
            request,
            before,
            decision,
            planned_actions,
            None,
        ));
    }

    let mutation_required = match &decision {
        SourceRepositoryBootstrapDecision::Converged => false,
        SourceRepositoryBootstrapDecision::BootstrapRequired
        | SourceRepositoryBootstrapDecision::SyncRequired => true,
        SourceRepositoryBootstrapDecision::Blocked { .. }
        | SourceRepositoryBootstrapDecision::Unknown { .. } => {
            let status = match &decision {
                SourceRepositoryBootstrapDecision::Blocked { .. } => {
                    SourceRepositoryBootstrapStatus::Blocked
                }
                _ => SourceRepositoryBootstrapStatus::Unknown,
            };
            return emit_report(report(
                status,
                request,
                before,
                decision,
                planned_actions,
                None,
            ));
        }
    };
    let api_key = match std::env::var("CYNIC_KERNEL_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
    {
        Some(api_key) => api_key,
        None => {
            let result = report(
                SourceRepositoryBootstrapStatus::Unknown,
                request,
                before,
                decision,
                planned_actions,
                None,
            );
            return emit_report(mark_unknown(
                result,
                "source_bootstrap_kernel_api_key_missing",
            ));
        }
    };
    let target = format!("source-repository:{node_id}:{repository_id}");
    let coordination = KernelSourceRepositoryProofCommit::new(kernel_endpoint, api_key);
    let guard = match coordination.claim(agent_id, &target).await {
        Ok(guard) => guard,
        Err(failure) => {
            let result = report(
                SourceRepositoryBootstrapStatus::Unknown,
                request,
                before,
                decision,
                planned_actions,
                None,
            );
            return emit_report(mark_unknown(
                result,
                format!("{}:{}", failure.code, failure.message),
            ));
        }
    };
    match coordination.verify(&guard).await {
        Ok(verification) if verification.status == "proved" => {}
        Ok(verification) => {
            let _ = coordination.release(&guard).await;
            let mut result = report(
                SourceRepositoryBootstrapStatus::Unknown,
                request,
                before,
                decision,
                planned_actions,
                None,
            );
            result.lease = Some(guard);
            return emit_report(mark_unknown(
                result,
                format!(
                    "source_bootstrap_lease_not_proved:{}",
                    verification.blockers.join(",")
                ),
            ));
        }
        Err(failure) => {
            let _ = coordination.release(&guard).await;
            let mut result = report(
                SourceRepositoryBootstrapStatus::Unknown,
                request,
                before,
                decision,
                planned_actions,
                None,
            );
            result.lease = Some(guard);
            return emit_report(mark_unknown(
                result,
                format!("{}:{}", failure.code, failure.message),
            ));
        }
    }
    let after = if mutation_required {
        if let Err(failure) = adapter.apply(&layout, &request) {
            let _ = coordination.release(&guard).await;
            let mut result = report(
                SourceRepositoryBootstrapStatus::Unknown,
                request,
                before,
                decision,
                planned_actions,
                None,
            );
            result.lease = Some(guard);
            result.blockers.push(format!(
                "source_bootstrap_apply:{}:{}",
                failure.code, failure.message
            ));
            return emit_report(result);
        }
        match adapter.observe(&layout, &request) {
            Ok(observation) => observation,
            Err(failure) => {
                let _ = coordination.release(&guard).await;
                let mut result = report(
                    SourceRepositoryBootstrapStatus::Unknown,
                    request,
                    before,
                    decision,
                    planned_actions,
                    None,
                );
                result.lease = Some(guard);
                return emit_report(mark_unknown(
                    result,
                    format!(
                        "source_bootstrap_read_back:{}:{}",
                        failure.code, failure.message
                    ),
                ));
            }
        }
    } else {
        before.clone()
    };
    let after_decision = reconcile_source_repository_bootstrap(&request, &after);
    let converged = matches!(
        &after_decision,
        SourceRepositoryBootstrapDecision::Converged
    );
    let status = match &after_decision {
        SourceRepositoryBootstrapDecision::Converged => SourceRepositoryBootstrapStatus::Completed,
        SourceRepositoryBootstrapDecision::Blocked { .. } => {
            SourceRepositoryBootstrapStatus::Blocked
        }
        _ => SourceRepositoryBootstrapStatus::Unknown,
    };
    let mut result = report(
        status,
        request,
        before,
        after_decision,
        planned_actions,
        Some(after),
    );
    result.lease = Some(guard.clone());
    if !converged {
        let _ = coordination.release(&guard).await;
        return emit_report(result);
    }
    let observed_at_unix_ms = current_unix_ms();
    result = persist_local_proof(result, &state_root_path, observed_at_unix_ms);
    if result.proof_authority != SourceRepositoryBootstrapProofAuthority::LocalContentAddressed {
        let _ = coordination.release(&guard).await;
        return emit_report(result);
    }
    let bundle = match runtime_proof_bundle(&result, &guard, observed_at_unix_ms) {
        Ok(bundle) => bundle,
        Err(failure) => {
            let _ = coordination.release(&guard).await;
            return emit_report(mark_unknown(result, failure));
        }
    };
    match coordination.finalize(&guard, &bundle).await {
        Ok(proof_ref) => emit_report(mark_kernel_committed(result, proof_ref)),
        Err(failure) => {
            let _ = coordination.release(&guard).await;
            emit_report(mark_unknown(
                result,
                format!("{}:{}", failure.code, failure.message),
            ))
        }
    }
}

fn report(
    status: SourceRepositoryBootstrapStatus,
    request: SourceRepositoryBootstrapRequest,
    before: cynic_protocol::SourceRepositoryBootstrapObservation,
    decision: SourceRepositoryBootstrapDecision,
    planned_actions: Vec<Vec<String>>,
    after: Option<cynic_protocol::SourceRepositoryBootstrapObservation>,
) -> SourceRepositoryBootstrapReport {
    let (blockers, unknowns) = match &decision {
        SourceRepositoryBootstrapDecision::Blocked { blockers } => (blockers.clone(), Vec::new()),
        SourceRepositoryBootstrapDecision::Unknown { unknowns } => (Vec::new(), unknowns.clone()),
        _ => (Vec::new(), Vec::new()),
    };
    SourceRepositoryBootstrapReport {
        schema: SOURCE_REPOSITORY_BOOTSTRAP_REPORT_SCHEMA.to_string(),
        status,
        request,
        before,
        decision,
        planned_actions,
        after,
        lease: None,
        proof_ref: None,
        proof_authority: SourceRepositoryBootstrapProofAuthority::Uncommitted,
        evidence_refs: Vec::new(),
        blockers,
        unknowns,
    }
}

fn current_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}

fn emit_report(report: SourceRepositoryBootstrapReport) -> ExitCode {
    let success = matches!(
        report.status,
        SourceRepositoryBootstrapStatus::Planned | SourceRepositoryBootstrapStatus::Completed
    );
    match serde_json::to_string_pretty(&report) {
        Ok(payload) => println!("{payload}"),
        Err(failure) => {
            eprintln!("source bootstrap report serialization failed: {failure}");
            return ExitCode::FAILURE;
        }
    }
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn emit_error(blocker: String) -> ExitCode {
    println!(
        "{}",
        serde_json::json!({
            "schema": SOURCE_REPOSITORY_BOOTSTRAP_REPORT_SCHEMA,
            "status": "blocked",
            "blockers": [blocker],
            "unknowns": []
        })
    );
    ExitCode::FAILURE
}
