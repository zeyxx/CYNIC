//! Bounded storage topology apply — typed argv, injected execution, re-observe proof.

use cynic_storage::client_policy::StorageClientPolicy;
use cynic_storage::topology::{
    STORAGE_TOPOLOGY_REPORT_SCHEMA, StorageTopologyDesired, StorageTopologyObserved,
    StorageTopologyReport,
};

use cynic_lifecycle::ArgvRunner;

use super::probe::{
    StorageClientPolicyObservationPort, StorageTopologyObservationPort, storage_topology_report,
};
use super::reconcile::{StorageTopologyAction, StorageTopologyPlan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageTopologyArgvAction {
    pub argv: Vec<String>,
    pub mutates: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageTopologyApplyIntent {
    pub actions: Vec<StorageTopologyArgvAction>,
    pub rollback: String,
    pub proof_ref: String,
    pub mutates: bool,
}

pub fn plan_storage_topology_apply(
    desired: &StorageTopologyDesired,
    observed: &StorageTopologyObserved,
    plan: &StorageTopologyPlan,
    _client_policy: &StorageClientPolicy,
    policy_path: &str,
) -> StorageTopologyApplyIntent {
    let actions = plan
        .actions
        .iter()
        .map(|action| StorageTopologyArgvAction {
            // No replica/leader argv is emitted for standalone backend: unsupported topology remains blocked.
            argv: match action {
                StorageTopologyAction::UpdateClientPolicy { endpoints } => {
                    let mut argv = vec![
                        "cynic".to_string(),
                        "runtime".to_string(),
                        "storage-client-policy-set".to_string(),
                        "--policy-path".to_string(),
                        policy_path.to_string(),
                    ];
                    for endpoint in endpoints {
                        argv.push("--endpoint".to_string());
                        argv.push(endpoint.clone());
                    }
                    argv.push("--apply".to_string());
                    argv
                }
                _ => Vec::new(),
            },
            mutates: matches!(action, StorageTopologyAction::UpdateClientPolicy { .. }),
        })
        .collect::<Vec<_>>();
    let rollback = if desired.target_topology.replicas == 1 {
        "restore single endpoint".to_string()
    } else {
        format!("rollback to {} replicas", observed.endpoints.len())
    };
    StorageTopologyApplyIntent {
        mutates: actions.iter().any(|action| action.mutates),
        actions,
        rollback,
        proof_ref: format!("storage_topology:{}", observed.endpoints.len()),
    }
}

pub fn apply_client_policy_file(
    path: &std::path::Path,
    policy: &StorageClientPolicy,
    apply: bool,
) -> Result<(), String> {
    if !apply {
        return Err("apply_not_admitted".to_string());
    }
    let payload = serde_json::to_vec_pretty(policy)
        .map_err(|error| format!("client_policy_encode_failed:{error}"))?;
    std::fs::write(path, payload)
        .map_err(|error| format!("client_policy_write_failed:{}:{error}", path.display()))?;
    let read_back = std::fs::read_to_string(path)
        .map_err(|error| format!("client_policy_readback_failed:{}:{error}", path.display()))?;
    let observed: StorageClientPolicy = serde_json::from_str(&read_back)
        .map_err(|error| format!("client_policy_readback_invalid:{error}"))?;
    if &observed == policy {
        Ok(())
    } else {
        Err("client_policy_readback_mismatch".to_string())
    }
}

pub fn apply_and_prove_storage_topology<
    R: ArgvRunner,
    O: StorageTopologyObservationPort,
    P: StorageClientPolicyObservationPort,
>(
    intent: &StorageTopologyApplyIntent,
    desired: &StorageTopologyDesired,
    observer: &O,
    policy_observer: &P,
    runner: &R,
    apply: bool,
) -> StorageTopologyReport {
    let pre = observer.observe_topology();
    let pre_digest = format!("observed:{:?}", pre.endpoints);
    let pre_policy = policy_observer.observe_client_policy();
    let pre_policy_digest = format!("client_policy:{:?}", pre_policy.client_policy.endpoints);
    let mut residual = Vec::new();
    let mut applied = Vec::new();
    if intent.mutates && !apply {
        residual.push("apply_not_admitted".to_string());
    }
    if apply {
        for action in intent.actions.iter().filter(|action| action.mutates) {
            let argv = action.argv.iter().map(String::as_str).collect::<Vec<_>>();
            if runner.run(&argv).success() {
                applied.push(action.argv.clone());
            } else {
                residual.push("apply_argv_failed".to_string());
            }
        }
    }
    let post = observer.observe_topology();
    let post_digest = format!("observed:{:?}", post.endpoints);
    let post_policy = policy_observer.observe_client_policy();
    let post_policy_digest = format!("client_policy:{:?}", post_policy.client_policy.endpoints);
    let mut report = storage_topology_report(desired, &post, &post_policy);
    report.schema = STORAGE_TOPOLOGY_REPORT_SCHEMA.to_string();
    report.pre_observed_digest = Some(format!("{pre_digest}|{pre_policy_digest}"));
    report.post_observed_digest = Some(format!("{post_digest}|{post_policy_digest}"));
    report.actions_applied = applied;
    report.residual_unknowns.extend(residual);
    if pre_digest == post_digest && pre_policy_digest == post_policy_digest && intent.mutates {
        report
            .residual_unknowns
            .push("post_apply_readback_unchanged".to_string());
    }
    report.residual_unknowns.sort();
    report.residual_unknowns.dedup();
    report.status = if report.residual_unknowns.is_empty() {
        "proven".to_string()
    } else {
        "blocked".to_string()
    };
    report.proof_ref = Some(intent.proof_ref.clone());
    report
}
