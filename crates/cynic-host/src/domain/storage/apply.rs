//! Bounded storage topology apply — typed argv, injected execution, re-observe proof.

use cynic_protocol::storage_client_policy::StorageClientPolicy;
use cynic_protocol::storage_topology::{
    STORAGE_TOPOLOGY_REPORT_SCHEMA, StorageTopologyDesired, StorageTopologyObserved,
    StorageTopologyReport,
};

use crate::update_lifecycle::ArgvRunner;

use super::probe::{StorageTopologyObservationPort, storage_topology_report};
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
) -> StorageTopologyApplyIntent {
    let actions = plan
        .actions
        .iter()
        .map(|action| StorageTopologyArgvAction {
            // No replica/leader argv is emitted for standalone backend: unsupported topology remains blocked.
            argv: match action {
                StorageTopologyAction::UpdateClientPolicy { endpoints } => vec![
                    "cynic".to_string(),
                    "storage".to_string(),
                    "client-policy".to_string(),
                    "set-endpoints".to_string(),
                    endpoints.join(","),
                ],
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

pub fn apply_and_prove_storage_topology<R: ArgvRunner, O: StorageTopologyObservationPort>(
    intent: &StorageTopologyApplyIntent,
    desired: &StorageTopologyDesired,
    client_policy: &StorageClientPolicy,
    observer: &O,
    runner: &R,
    apply: bool,
) -> StorageTopologyReport {
    let pre = observer.observe_topology();
    let pre_digest = format!("observed:{}", pre.endpoints.len());
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
    let post_digest = format!("observed:{}", post.endpoints.len());
    let mut report = storage_topology_report(desired, &post, client_policy);
    report.schema = STORAGE_TOPOLOGY_REPORT_SCHEMA.to_string();
    report.pre_observed_digest = Some(pre_digest.clone());
    report.post_observed_digest = Some(post_digest.clone());
    report.actions_applied = applied;
    report.residual_unknowns.extend(residual);
    if pre_digest == post_digest && intent.mutates {
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
