use cynic_protocol::{
    MutationContract, PolicyAdmissionReport, PolicyAuthority, PolicyDecision, PolicyIntentRef,
    PolicyScope, PolicySetRef, RequiredProof,
};

use super::admission::{HOST_OPERATION_PLAN_SCHEMA, HostOperationPlan, digest_json};
use crate::domain::principal::PrincipalPolicy;

const PRINCIPAL_OPERATION_POLICY_ID: &str = "cynic.host.principal.execution";
const PRINCIPAL_OPERATION_POLICY_VERSION: &str = "v1";

pub(crate) fn build_principal_operation_plan(
    policy: &PrincipalPolicy,
    target: impl Into<String>,
) -> HostOperationPlan {
    let target = target.into();
    let mut expected_proofs = vec![
        proof("principal_user_present", &target),
        proof("principal_group_present", &target),
        proof("principal_home_matches", &target),
    ];
    expected_proofs.extend(
        policy
            .state_dirs
            .iter()
            .map(|path| proof(&format!("state_dir_present:{path}"), &target)),
    );
    expected_proofs.extend(
        policy
            .managed_paths()
            .iter()
            .map(|path| proof(&format!("path_owned:{path}"), &target)),
    );

    HostOperationPlan {
        schema: HOST_OPERATION_PLAN_SCHEMA,
        id: format!("host.principal.prepare:{target}"),
        kind: "principal_prepare".to_string(),
        domain: "principal".to_string(),
        target,
        mutates: true,
        requires_privilege: true,
        operations: policy.apply_operations(),
        preconditions: vec!["principal_policy_declared".to_string()],
        rollback_intent:
            "remove principal only after disabling services and removing exclusively owned paths"
                .to_string(),
        expected_proofs,
    }
}

pub(crate) fn principal_operation_policy_admission(
    plan: &HostOperationPlan,
    actor: impl Into<String>,
) -> PolicyAdmissionReport {
    let actor = actor.into();
    let plan_digest = digest_json(plan);
    let mutation = MutationContract {
        kind: plan.kind.clone(),
        scope: PolicyScope {
            domain: plan.domain.clone(),
            target: plan.target.clone(),
        },
        mutates: plan.mutates,
        requires_privilege: plan.requires_privilege,
        requires_explicit_apply: true,
        required_authority: PolicyAuthority::HumanAdmin,
        preconditions: plan.preconditions.clone(),
        rollback_intent: plan.rollback_intent.clone(),
        expected_proofs: plan.expected_proofs.clone(),
    };
    PolicyAdmissionReport::new(
        PolicySetRef {
            id: PRINCIPAL_OPERATION_POLICY_ID.to_string(),
            version: PRINCIPAL_OPERATION_POLICY_VERSION.to_string(),
            digest: digest_json(&principal_policy_descriptor()),
        },
        PolicyIntentRef {
            id: plan.id.clone(),
            kind: plan.kind.clone(),
            actor,
            subject: plan.target.clone(),
            digest: plan_digest.clone(),
            plan_digest,
        },
        mutation.clone(),
        PolicyDecision::Allow {
            required_authority: mutation.required_authority,
            required_proofs: mutation.expected_proofs.clone(),
        },
    )
}

fn proof(gate: &str, target: &str) -> RequiredProof {
    RequiredProof {
        gate: gate.to_string(),
        subject: target.to_string(),
    }
}

fn principal_policy_descriptor() -> serde_json::Value {
    serde_json::json!({
        "id": PRINCIPAL_OPERATION_POLICY_ID,
        "version": PRINCIPAL_OPERATION_POLICY_VERSION,
        "kind": "principal_prepare",
        "domain": "principal",
        "required_authority": "human_admin",
        "requires_explicit_apply": true,
        "requires_privilege": true,
        "requires_rollback": true,
        "requires_postflight_proof": true
    })
}
