use cynic_protocol::{
    EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization, MutationContract,
    POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority, PolicyDecision,
    PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof,
};
use serde::Serialize;

use crate::domain::services::ServicePolicy;
use crate::ports::HostOperation;

pub const HOST_OPERATION_PLAN_SCHEMA: &str = "cynic.host.operation_plan.v1";
const SERVICE_OPERATION_POLICY_ID: &str = "cynic.host.services.execution";
const SERVICE_OPERATION_POLICY_VERSION: &str = "v1";
const RUNTIME_SERVICE_OPERATION_POLICY_ID: &str = "cynic.host.services.runtime_reconcile";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostOperationPlan {
    pub schema: &'static str,
    pub id: String,
    pub kind: String,
    pub domain: String,
    pub target: String,
    pub mutates: bool,
    pub requires_privilege: bool,
    pub operations: Vec<HostOperation>,
    pub preconditions: Vec<String>,
    pub rollback_intent: String,
    pub expected_proofs: Vec<RequiredProof>,
}

#[derive(Debug, Clone)]
pub struct AdmittedHostOperation {
    operation: HostOperation,
    plan_digest: String,
}

impl AdmittedHostOperation {
    pub fn operation(&self) -> &HostOperation {
        &self.operation
    }

    pub fn plan_digest(&self) -> &str {
        &self.plan_digest
    }
}

#[derive(Debug, Clone)]
pub struct AdmittedHostOperationPlan {
    pub plan: HostOperationPlan,
    pub admission: PolicyAdmissionReport,
    pub authorization: ExecutionAuthorization,
    operations: Vec<AdmittedHostOperation>,
}

impl AdmittedHostOperationPlan {
    pub fn operations(&self) -> &[AdmittedHostOperation] {
        &self.operations
    }

    pub fn plan_digest(&self) -> String {
        digest_json(&self.plan)
    }
}

pub(crate) fn build_service_operation_plan(
    policy: &ServicePolicy,
    target: impl Into<String>,
) -> HostOperationPlan {
    let target = target.into();
    let expected_proofs = policy
        .units
        .iter()
        .flat_map(|unit| {
            let mut proofs = Vec::new();
            if unit.enable_on_boot {
                proofs.push(RequiredProof {
                    gate: format!("unit_enabled:{}", unit.unit_name),
                    subject: target.clone(),
                });
            }
            proofs.push(RequiredProof {
                gate: format!("unit_active:{}", unit.unit_name),
                subject: target.clone(),
            });
            proofs
        })
        .collect();

    HostOperationPlan {
        schema: HOST_OPERATION_PLAN_SCHEMA,
        id: format!("host.services.apply:{target}"),
        kind: "service_control".to_string(),
        domain: "services".to_string(),
        target,
        mutates: true,
        requires_privilege: true,
        operations: policy.apply_operations(),
        preconditions: vec![
            "host_principal_active".to_string(),
            "host_privilege_active".to_string(),
            "host_packages_active".to_string(),
            "service_policy_declared".to_string(),
        ],
        rollback_intent: "restore previous service enablement and active state".to_string(),
        expected_proofs,
    }
}

pub(crate) fn service_operation_policy_admission(
    plan: &HostOperationPlan,
    actor: impl Into<String>,
) -> PolicyAdmissionReport {
    service_operation_policy_admission_with_authority(
        plan,
        actor,
        PolicyAuthority::HumanOperator,
        SERVICE_OPERATION_POLICY_ID,
    )
}

pub(crate) fn runtime_service_operation_policy_admission(
    plan: &HostOperationPlan,
    actor: impl Into<String>,
) -> PolicyAdmissionReport {
    service_operation_policy_admission_with_authority(
        plan,
        actor,
        PolicyAuthority::RuntimeAuthority,
        RUNTIME_SERVICE_OPERATION_POLICY_ID,
    )
}

fn service_operation_policy_admission_with_authority(
    plan: &HostOperationPlan,
    actor: impl Into<String>,
    required_authority: PolicyAuthority,
    policy_id: &str,
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
        required_authority,
        preconditions: plan.preconditions.clone(),
        rollback_intent: plan.rollback_intent.clone(),
        expected_proofs: plan.expected_proofs.clone(),
    };
    PolicyAdmissionReport::new(
        PolicySetRef {
            id: policy_id.to_string(),
            version: SERVICE_OPERATION_POLICY_VERSION.to_string(),
            digest: digest_json(&service_policy_descriptor(policy_id, required_authority)),
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

pub(crate) fn admit_host_operation_plan(
    plan: HostOperationPlan,
    admission: PolicyAdmissionReport,
    authorization: ExecutionAuthorization,
) -> Result<AdmittedHostOperationPlan, Vec<String>> {
    let mut blockers = Vec::new();
    let plan_digest = digest_json(&plan);

    check(
        plan.schema == HOST_OPERATION_PLAN_SCHEMA,
        "plan_schema_invalid",
        &mut blockers,
    );
    check(
        admission.schema == POLICY_ADMISSION_REPORT_SCHEMA,
        "admission_schema_invalid",
        &mut blockers,
    );
    check(
        authorization.schema == EXECUTION_AUTHORIZATION_SCHEMA,
        "authorization_schema_invalid",
        &mut blockers,
    );
    check(
        admission.admitted(),
        "policy_decision_not_allow",
        &mut blockers,
    );
    check(
        admission.intent.plan_digest == plan_digest,
        "plan_digest_mismatch",
        &mut blockers,
    );
    check(
        admission.intent.kind == plan.kind
            && admission.intent.subject == plan.target
            && admission.intent.actor == authorization.actor,
        "intent_binding_mismatch",
        &mut blockers,
    );
    check(
        admission.mutation.kind == plan.kind
            && admission.mutation.scope.domain == plan.domain
            && admission.mutation.scope.target == plan.target
            && admission.mutation.mutates == plan.mutates
            && admission.mutation.requires_privilege == plan.requires_privilege,
        "mutation_contract_mismatch",
        &mut blockers,
    );
    check(
        admission.mutation.preconditions == plan.preconditions,
        "precondition_contract_mismatch",
        &mut blockers,
    );
    check(
        admission.mutation.rollback_intent == plan.rollback_intent
            && (!plan.mutates || !plan.rollback_intent.trim().is_empty()),
        "rollback_contract_missing",
        &mut blockers,
    );
    check(
        admission.mutation.expected_proofs == plan.expected_proofs
            && (!plan.mutates || !plan.expected_proofs.is_empty()),
        "proof_contract_mismatch",
        &mut blockers,
    );
    check(
        !plan.operations.is_empty(),
        "operation_plan_empty",
        &mut blockers,
    );
    check(
        !admission.policy_set.digest.ends_with("unavailable")
            && admission.policy_set.digest.starts_with("sha256:"),
        "policy_digest_invalid",
        &mut blockers,
    );

    match &admission.decision {
        PolicyDecision::Allow {
            required_authority,
            required_proofs,
        } => {
            check(
                *required_authority == admission.mutation.required_authority
                    && authorization.authority == *required_authority,
                "authority_insufficient",
                &mut blockers,
            );
            check(
                required_proofs == &admission.mutation.expected_proofs,
                "decision_proof_mismatch",
                &mut blockers,
            );
        }
        PolicyDecision::Deny { .. } | PolicyDecision::Unknown { .. } => {}
    }

    if admission.mutation.requires_explicit_apply {
        check(
            authorization.explicit_apply,
            "explicit_apply_required",
            &mut blockers,
        );
    }
    for precondition in &plan.preconditions {
        check(
            authorization.satisfied_preconditions.contains(precondition),
            &format!("precondition_unsatisfied:{precondition}"),
            &mut blockers,
        );
    }

    if !blockers.is_empty() {
        return Err(blockers);
    }

    let operations = plan
        .operations
        .iter()
        .cloned()
        .map(|operation| AdmittedHostOperation {
            operation,
            plan_digest: plan_digest.clone(),
        })
        .collect();
    Ok(AdmittedHostOperationPlan {
        plan,
        admission,
        authorization,
        operations,
    })
}

fn check(condition: bool, blocker: &str, blockers: &mut Vec<String>) {
    if !condition {
        blockers.push(blocker.to_string());
    }
}

pub(crate) fn digest_json<T: Serialize>(value: &T) -> String {
    cynic_protocol::digest_json(value).unwrap_or_else(|_| "sha256:unavailable".to_string())
}

pub(crate) fn stderr_with_spawn_error(mut stderr: String, spawn_error: Option<String>) -> String {
    if let Some(error) = spawn_error {
        if stderr.is_empty() {
            stderr = error;
        } else {
            stderr = format!("{error}: {stderr}");
        }
    }
    stderr
}

fn service_policy_descriptor(
    policy_id: &str,
    required_authority: PolicyAuthority,
) -> serde_json::Value {
    serde_json::json!({
        "id": policy_id,
        "version": SERVICE_OPERATION_POLICY_VERSION,
        "kind": "service_control",
        "domain": "services",
        "required_authority": required_authority,
        "requires_explicit_apply": true,
        "requires_privilege": true,
        "requires_rollback": true,
        "requires_postflight_proof": true
    })
}
