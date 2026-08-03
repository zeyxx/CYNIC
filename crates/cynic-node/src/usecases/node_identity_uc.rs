use std::process::ExitCode;

use serde::Serialize;

use cynic_protocol::{
    NODE_IDENTITY_DESIRED_STATE_SCHEMA, NodeIdentityDecision, NodeIdentityDesiredState,
    NodeIdentityObservation, NodeIdentityPlan, reconcile_node_identity,
};

use crate::adapters::local_node_identity::LocalNodeIdentity;
use crate::ports::node_identity_port::NodeIdentityPort;

#[derive(Debug, Serialize)]
struct NodeIdentityReport {
    schema: &'static str,
    status: &'static str,
    mutates: bool,
    requested_apply: bool,
    desired: NodeIdentityDesiredState,
    observation_before: NodeIdentityObservation,
    plan: NodeIdentityPlan,
    observation_after: Option<NodeIdentityObservation>,
    blockers: Vec<String>,
    rollback_intent: &'static str,
}

pub fn execute_node_identity(node_id: &str, signing_key_path: &str, apply: bool) -> ExitCode {
    let desired = NodeIdentityDesiredState {
        schema: NODE_IDENTITY_DESIRED_STATE_SCHEMA.to_string(),
        node_id: node_id.to_string(),
        signing_key_path: signing_key_path.to_string(),
    };
    let adapter = LocalNodeIdentity;
    let before = adapter.observe(&desired);
    let plan = reconcile_node_identity(&desired, &before);
    if !apply || plan.decision != NodeIdentityDecision::Generate {
        let successful = matches!(
            plan.decision,
            NodeIdentityDecision::Generate | NodeIdentityDecision::AlreadyPresent
        );
        let report = NodeIdentityReport {
            schema: "cynic.node_identity.report.v1",
            status: match plan.decision {
                NodeIdentityDecision::Generate => "planned",
                NodeIdentityDecision::AlreadyPresent => "converged",
                NodeIdentityDecision::Unknown => "unknown",
                NodeIdentityDecision::Blocked => "blocked",
            },
            mutates: false,
            requested_apply: apply,
            desired,
            observation_before: before,
            blockers: plan
                .blockers
                .iter()
                .chain(&plan.unknowns)
                .cloned()
                .collect(),
            plan,
            observation_after: None,
            rollback_intent: "retain an existing identity; no mutation was attempted",
        };
        return emit(&report, successful);
    }

    if let Err(error) = adapter.generate(&desired) {
        let report = NodeIdentityReport {
            schema: "cynic.node_identity.report.v1",
            status: "blocked",
            mutates: true,
            requested_apply: true,
            desired,
            observation_before: before,
            plan,
            observation_after: None,
            blockers: vec![error],
            rollback_intent: "remove only an unpublished temporary key; never replace an existing identity",
        };
        return emit(&report, false);
    }
    let after = adapter.observe(&desired);
    let read_back = reconcile_node_identity(&desired, &after);
    let converged = read_back.decision == NodeIdentityDecision::AlreadyPresent;
    let report = NodeIdentityReport {
        schema: "cynic.node_identity.report.v1",
        status: if converged { "converged" } else { "blocked" },
        mutates: true,
        requested_apply: true,
        desired,
        observation_before: before,
        blockers: read_back
            .blockers
            .iter()
            .chain(&read_back.unknowns)
            .cloned()
            .collect(),
        plan,
        observation_after: Some(after),
        rollback_intent: "remove the newly generated identity only before any enrollment grant binds it",
    };
    emit(&report, converged)
}

fn emit(value: &impl Serialize, success: bool) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("node identity report serialization failed: {error}");
            return ExitCode::FAILURE;
        }
    }
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
