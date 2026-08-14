use serde::{Deserialize, Serialize};
use std::path::Path;

use cynic_protocol::{
    ExecutionAuthorization, MutationContract, PolicyAdmissionReport, PolicyAuthority,
    PolicyDecision, PolicyIntentRef, PolicyScope, PolicySetRef, PolicyViolation, RequiredProof,
};

use super::fleet_truth::ObservedResource;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetDesiredTarget {
    pub id: String,
    pub provider: String,
    pub resource_id: String,
    pub role: String,
    pub desired_power: DesiredPower,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesiredPower {
    Running,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FleetReconcileAction {
    Start {
        provider: String,
        resource_id: String,
    },
    Stop {
        provider: String,
        resource_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FleetReconcilePlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub desired_target: String,
    pub role: String,
    pub observed_status: Option<String>,
    pub actions: Vec<FleetReconcileAction>,
    pub blockers: Vec<String>,
    pub expected_proofs: Vec<&'static str>,
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
            gate: (*gate).to_string(),
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
        preconditions: vec!["provider_resource_observed".to_string()],
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
            version: "v1".to_string(),
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
    observed: &[ObservedResource],
) -> FleetReconcilePlan {
    let resource = observed.iter().find(|resource| {
        resource.provider == desired.provider
            && resource.resource_id.as_deref() == Some(&desired.resource_id)
    });
    let observed_status = resource.map(|resource| resource.status.clone());
    let mut blockers = Vec::new();
    let mut actions = Vec::new();
    match resource {
        None => blockers.push("desired_resource_unobserved".to_string()),
        Some(resource) => match desired.desired_power {
            DesiredPower::Running if resource.status != "running" => {
                actions.push(FleetReconcileAction::Start {
                    provider: desired.provider.clone(),
                    resource_id: desired.resource_id.clone(),
                })
            }
            DesiredPower::Stopped if resource.status == "running" => {
                actions.push(FleetReconcileAction::Stop {
                    provider: desired.provider.clone(),
                    resource_id: desired.resource_id.clone(),
                })
            }
            _ => {}
        },
    }
    let status = if blockers.is_empty() {
        "planned"
    } else {
        "blocked"
    };
    FleetReconcilePlan {
        schema: "cynic.fleet.reconcile_plan.v1",
        status,
        mutates: false,
        desired_target: desired.id.clone(),
        role: desired.role.clone(),
        observed_status,
        actions,
        blockers,
        expected_proofs: vec!["provider_action_read_back", "desired_power_observed"],
        rollback_intent: "restore the prior observed power state through the same admitted provider adapter",
    }
}

pub fn load_desired_targets(path: &Path) -> Result<Vec<FleetDesiredTarget>, String> {
    let bytes = std::fs::read(path).map_err(|error| {
        format!(
            "failed to read fleet desired state {}: {error}",
            path.display()
        )
    })?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "failed to parse fleet desired state {}: {error}",
            path.display()
        )
    })?;
    let targets = value
        .pointer("/fleet_desired/targets")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    serde_json::from_value(targets)
        .map_err(|error| format!("fleet_desired.targets is invalid: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desired() -> FleetDesiredTarget {
        FleetDesiredTarget {
            id: "kernel-canary".into(),
            provider: "proxmox".into(),
            resource_id: "qemu/120".into(),
            role: "kernel".into(),
            desired_power: DesiredPower::Running,
        }
    }

    #[test]
    fn stopped_observed_target_plans_start_without_mutating() {
        let observed = vec![ObservedResource {
            provider: "proxmox".into(),
            name: "cynic-vm-core-probe".into(),
            status: "stopped".into(),
            resource_id: Some("qemu/120".into()),
            kind: Some("qemu".into()),
            constellation: None,
        }];
        let plan = build_plan(&desired(), &observed);
        assert_eq!(plan.status, "planned");
        assert!(!plan.mutates);
        assert_eq!(plan.actions.len(), 1);
    }

    #[test]
    fn missing_target_cannot_produce_an_apply_action() {
        let plan = build_plan(&desired(), &[]);
        assert_eq!(plan.status, "blocked");
        assert!(plan.actions.is_empty());
        assert!(
            plan.blockers
                .contains(&"desired_resource_unobserved".to_string())
        );
    }

    #[test]
    fn desired_targets_are_loaded_from_registry_not_hardcoded() {
        let file = tempfile::NamedTempFile::new().expect("registry");
        std::fs::write(file.path(), r#"{"fleet_desired":{"targets":[{"id":"kernel-canary","provider":"proxmox","resource_id":"qemu/120","role":"kernel","desired_power":"running"}]}}"#).expect("write");
        let targets = load_desired_targets(file.path()).expect("targets");
        assert_eq!(targets[0].resource_id, "qemu/120");
    }

    #[test]
    fn apply_requires_matching_admission_and_explicit_consent() {
        let observed = vec![ObservedResource {
            provider: "proxmox".into(),
            name: "core".into(),
            status: "stopped".into(),
            resource_id: Some("qemu/120".into()),
            kind: None,
            constellation: None,
        }];
        let plan = build_plan(&desired(), &observed);
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
