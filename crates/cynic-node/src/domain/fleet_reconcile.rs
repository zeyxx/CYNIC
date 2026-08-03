use serde::Serialize;
use std::path::Path;

use cynic_host::registry::{ResolvedNodeSubstrateTarget, resolve_node_substrate_targets};
use cynic_protocol::{
    ExecutionAuthorization, MutationContract, NodeSubstrateAction, NodeSubstrateDecision,
    NodeSubstrateObservation, PolicyAdmissionReport, PolicyAuthority, PolicyDecision,
    PolicyIntentRef, PolicyScope, PolicySetRef, PolicyViolation, RequiredProof,
    reconcile_node_substrate,
};

pub type FleetDesiredTarget = ResolvedNodeSubstrateTarget;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FleetReconcilePlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub desired_target: String,
    pub role: String,
    pub desired: cynic_protocol::NodeSubstrateDesiredState,
    pub observed: NodeSubstrateObservation,
    pub decision: NodeSubstrateDecision,
    pub actions: Vec<NodeSubstrateAction>,
    pub blockers: Vec<String>,
    pub expected_proofs: Vec<String>,
    pub rollback_intent: &'static str,
}

pub fn plan_digest(plan: &FleetReconcilePlan) -> String {
    cynic_protocol::digest_json(plan).expect("fleet reconcile plan is serializable")
}

pub fn admit_plan(plan: &FleetReconcilePlan, actor: String) -> PolicyAdmissionReport {
    let digest = plan_digest(plan);
    let required_proofs = plan
        .expected_proofs
        .iter()
        .map(|gate| RequiredProof {
            gate: gate.clone(),
            subject: plan.desired_target.clone(),
        })
        .collect::<Vec<_>>();
    let mutation = MutationContract {
        kind: "fleet_power_reconcile".to_string(),
        scope: PolicyScope {
            domain: "fleet".to_string(),
            target: plan.desired_target.clone(),
        },
        mutates: true,
        requires_privilege: true,
        requires_explicit_apply: true,
        required_authority: PolicyAuthority::RuntimeAuthority,
        preconditions: vec!["node_substrate_observation_proven".to_string()],
        rollback_intent: plan.rollback_intent.to_string(),
        expected_proofs: required_proofs.clone(),
    };
    let decision = if plan.status == "planned" && !plan.actions.is_empty() {
        PolicyDecision::Allow {
            required_authority: mutation.required_authority,
            required_proofs,
        }
    } else {
        PolicyDecision::Deny {
            violations: vec![violation("plan_not_actionable")],
        }
    };
    PolicyAdmissionReport::new(
        PolicySetRef {
            id: "cynic.fleet.power_reconcile".to_string(),
            version: "v2".to_string(),
            digest: cynic_protocol::digest_json(&mutation).expect("mutation is serializable"),
        },
        PolicyIntentRef {
            id: format!("fleet.reconcile:{}", plan.desired_target),
            kind: mutation.kind.clone(),
            actor,
            subject: plan.desired_target.clone(),
            digest: digest.clone(),
            plan_digest: digest,
        },
        mutation,
        decision,
    )
}

pub fn authorize_apply(
    plan: &FleetReconcilePlan,
    admission: &PolicyAdmissionReport,
    authorization: &ExecutionAuthorization,
) -> PolicyDecision {
    let digest = plan_digest(plan);
    let mut violations = Vec::new();
    if plan.status != "planned" || plan.actions.is_empty() {
        violations.push(violation("plan_not_actionable"));
    }
    if !admission.admitted() {
        violations.push(violation("admission_not_granted"));
    }
    if admission.intent.plan_digest != digest {
        violations.push(violation("admission_plan_digest_mismatch"));
    }
    if admission.intent.actor != authorization.actor {
        violations.push(violation("authorization_actor_mismatch"));
    }
    if authorization.authority != admission.mutation.required_authority {
        violations.push(violation("authorization_authority_mismatch"));
    }
    if !authorization.explicit_apply {
        violations.push(violation("explicit_apply_required"));
    }
    if violations.is_empty() {
        PolicyDecision::Allow {
            required_authority: admission.mutation.required_authority,
            required_proofs: admission.mutation.expected_proofs.clone(),
        }
    } else {
        PolicyDecision::Deny { violations }
    }
}

fn violation(code: &str) -> PolicyViolation {
    PolicyViolation {
        policy_id: "cynic.fleet.power_reconcile".to_string(),
        code: code.to_string(),
        message: code.replace('_', " "),
    }
}

pub fn build_plan(
    desired: &FleetDesiredTarget,
    observed: NodeSubstrateObservation,
) -> FleetReconcilePlan {
    let decision = reconcile_node_substrate(&desired.desired, &observed);
    let action = decision.action.clone();
    let power_action = action.filter(|action| {
        matches!(
            action,
            NodeSubstrateAction::Start | NodeSubstrateAction::Stop
        )
    });
    let mut blockers = decision.blockers.clone();
    if decision.status == "planned" && decision.mutates && power_action.is_none() {
        blockers.push("fleet_power_action_adapter_unavailable".to_string());
    }
    blockers.sort();
    blockers.dedup();
    let status = if decision.status == "converged" {
        "converged"
    } else if decision.status == "planned" && power_action.is_some() && blockers.is_empty() {
        "planned"
    } else {
        "blocked"
    };
    FleetReconcilePlan {
        schema: "cynic.fleet.reconcile_plan.v2",
        status,
        mutates: false,
        desired_target: desired.target_id.clone(),
        role: desired.role.clone(),
        desired: desired.desired.clone(),
        observed,
        expected_proofs: decision.required_read_back.clone(),
        decision,
        actions: power_action.into_iter().collect(),
        blockers,
        rollback_intent: "restore the prior observed power state through the same admitted provider adapter",
    }
}

pub fn load_desired_targets(path: &Path) -> Result<Vec<FleetDesiredTarget>, String> {
    resolve_node_substrate_targets(path.to_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        NODE_SUBSTRATE_OBSERVATION_SCHEMA, NodeNetworkAttachment, NodeSubstrateDesiredState,
        NodeSubstrateExistence, NodeSubstratePower,
    };

    fn desired() -> FleetDesiredTarget {
        FleetDesiredTarget {
            target_id: "kernel-canary".into(),
            role: "kernel".into(),
            desired: NodeSubstrateDesiredState {
                node_id: "kernel".into(),
                provider: Some("proxmox".into()),
                resource_id: Some("qemu/120".into()),
                desired_power: NodeSubstratePower::Running,
            },
        }
    }

    fn observed(power: NodeSubstratePower) -> NodeSubstrateObservation {
        NodeSubstrateObservation {
            schema: NODE_SUBSTRATE_OBSERVATION_SCHEMA.to_string(),
            status: "observed".to_string(),
            node_id: "kernel".to_string(),
            provider: Some("proxmox".to_string()),
            resource_id: Some("qemu/120".to_string()),
            existence: NodeSubstrateExistence::Present,
            power,
            network_attachment: NodeNetworkAttachment::Unknown,
            observed_addresses: Vec::new(),
            management_reachable: None,
            observation_authority: Some("provider:proxmox:api".to_string()),
            observed_at_unix_ms: Some(1),
            evidence_refs: vec!["provider_inventory_read_back".to_string()],
            unknowns: vec!["node_network_attachment_unknown".to_string()],
        }
    }

    #[test]
    fn stopped_observed_target_projects_canonical_start_decision() {
        let plan = build_plan(&desired(), observed(NodeSubstratePower::Stopped));
        assert_eq!(plan.status, "planned");
        assert!(!plan.mutates);
        assert_eq!(plan.actions, vec![NodeSubstrateAction::Start]);
        assert_eq!(
            plan.actions,
            plan.decision.action.clone().into_iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn unproven_observation_cannot_produce_an_apply_action() {
        let mut observation = observed(NodeSubstratePower::Unknown);
        observation.status = "unknown".to_string();
        observation.observation_authority = None;
        observation.observed_at_unix_ms = None;
        observation.evidence_refs.clear();
        let plan = build_plan(&desired(), observation);
        assert_eq!(plan.status, "blocked");
        assert!(plan.actions.is_empty());
        assert!(
            plan.blockers
                .contains(&"node_substrate_observation_unproven".to_string())
        );
    }

    #[test]
    fn apply_requires_matching_admission_and_explicit_consent() {
        let plan = build_plan(&desired(), observed(NodeSubstratePower::Stopped));
        let actor = "runtime-authority".to_string();
        let admission = admit_plan(&plan, actor.clone());
        let blocked = ExecutionAuthorization::new(
            actor.clone(),
            PolicyAuthority::RuntimeAuthority,
            false,
            Vec::new(),
        );
        let allowed =
            ExecutionAuthorization::new(actor, PolicyAuthority::RuntimeAuthority, true, Vec::new());
        assert!(!authorize_apply(&plan, &admission, &blocked).admitted());
        assert!(authorize_apply(&plan, &admission, &allowed).admitted());
    }
}
