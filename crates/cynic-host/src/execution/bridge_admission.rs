use cynic_protocol::{
    MutationContract, PolicyAdmissionReport, PolicyAuthority, PolicyDecision, PolicyIntentRef,
    PolicyScope, PolicySetRef, RequiredProof,
};

use crate::bridge::PrivilegeBridgeInstallSpec;
use crate::ports::HostOperation;

use super::admission::{HOST_OPERATION_PLAN_SCHEMA, HostOperationPlan, digest_json};

const POLICY_ID: &str = "cynic.host.privilege_bridge.execution";
const POLICY_VERSION: &str = "v1";

pub(crate) fn build_privilege_bridge_operation_plan(
    spec: &PrivilegeBridgeInstallSpec,
) -> HostOperationPlan {
    let target = spec.target.clone();
    HostOperationPlan {
        schema: HOST_OPERATION_PLAN_SCHEMA,
        id: format!("host.privilege_bridge.install:{target}"),
        kind: "privilege_bridge_install".to_string(),
        domain: "privilege".to_string(),
        target: target.clone(),
        mutates: true,
        requires_privilege: true,
        operations: vec![
            HostOperation::EnsureOwnedDir {
                path: parent_or_target(&spec.helper_target),
                user: spec.authority_user.clone(),
                group: spec.authority_group.clone(),
                mode: 0o755,
            },
            HostOperation::EnsureOwnedDir {
                path: parent_or_target(&spec.projection_target),
                user: spec.authority_user.clone(),
                group: spec.authority_group.clone(),
                mode: 0o755,
            },
            HostOperation::InstallOwnedFile {
                source: spec.helper_source.clone(),
                target: spec.helper_target.clone(),
                user: spec.authority_user.clone(),
                group: spec.authority_group.clone(),
                mode: spec.helper_mode,
            },
            HostOperation::InstallOwnedFile {
                source: spec.projection_source.clone(),
                target: spec.projection_target.clone(),
                user: spec.authority_user.clone(),
                group: spec.authority_group.clone(),
                mode: spec.projection_mode,
            },
            HostOperation::ValidatePrivilegeProjection {
                provider: spec.provider.clone(),
                path: spec.projection_target.clone(),
            },
        ],
        preconditions: privilege_bridge_preconditions(),
        rollback_intent:
            "remove the installed projection before the helper, preserving shared directories"
                .to_string(),
        expected_proofs: proof_gates()
            .into_iter()
            .map(|gate| RequiredProof {
                gate: gate.to_string(),
                subject: target.clone(),
            })
            .collect(),
    }
}

pub(crate) fn privilege_bridge_policy_admission(
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
            id: POLICY_ID.to_string(),
            version: POLICY_VERSION.to_string(),
            digest: digest_json(&policy_descriptor()),
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

pub fn privilege_bridge_preconditions() -> Vec<String> {
    vec![
        "runtime_principal_present".to_string(),
        "bridge_bundle_complete".to_string(),
        "privilege_projection_provider_available".to_string(),
    ]
}

pub(crate) fn proof_gates() -> [&'static str; 10] {
    [
        "helper_present",
        "helper_executable",
        "helper_mode_matches",
        "helper_authority_owned",
        "projection_present",
        "projection_mode_matches",
        "projection_authority_owned",
        "projection_valid",
        "invocation_policy_bounded",
        "bounded_helper_noninteractive_and_shell_denied",
    ]
}

fn parent_or_target(path: &str) -> String {
    std::path::Path::new(path)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.display().to_string())
        .unwrap_or_else(|| path.to_string())
}

fn policy_descriptor() -> serde_json::Value {
    serde_json::json!({
        "id": POLICY_ID,
        "version": POLICY_VERSION,
        "kind": "privilege_bridge_install",
        "domain": "privilege",
        "required_authority": "human_admin",
        "requires_explicit_apply": true,
        "requires_privilege": true,
        "provider_selected_by_port": true,
        "requires_rollback": true,
        "requires_postflight_proof": true
    })
}
