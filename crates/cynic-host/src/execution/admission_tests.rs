use cynic_protocol::{ExecutionAuthorization, PolicyAuthority, PolicyDecision};

use super::{admit_host_operation_plan, build_service_operation_plan};
use crate::domain::services::{ServiceManagerKind, ServicePolicy, ServiceUnitPolicy};
use crate::execution::{HostOperationPlan, service_operation_policy_admission};
use crate::ports::HostOperation;

fn plan() -> HostOperationPlan {
    build_service_operation_plan(
        &ServicePolicy {
            manager: ServiceManagerKind::Systemd,
            unit_dir: "/etc/systemd/system".to_string(),
            units: vec![ServiceUnitPolicy {
                service_id: "cynic-node".to_string(),
                unit_name: "cynic-node.service".to_string(),
                enable_on_boot: true,
                restart_mode: "on-failure".to_string(),
                source: "registry".to_string(),
                health_port: None,
                process_name: Some("cynic-node".to_string()),
                supervisor_provider: Some("systemd".to_string()),
                health_kind: Some("process".to_string()),
            }],
        },
        "node-a",
    )
}

fn authorization() -> ExecutionAuthorization {
    ExecutionAuthorization::new(
        "operator-a",
        PolicyAuthority::HumanOperator,
        true,
        plan().preconditions,
    )
}

#[test]
fn exact_plan_admission_mints_runner_tokens() {
    let plan = plan();
    let admission = service_operation_policy_admission(&plan, "operator-a");
    let admitted =
        admit_host_operation_plan(plan, admission, authorization()).expect("admitted service plan");

    assert_eq!(admitted.operations().len(), 3);
    assert!(
        admitted
            .operations()
            .iter()
            .all(|operation| operation.plan_digest() == admitted.plan_digest())
    );
}

#[test]
fn modified_plan_is_rejected() {
    let mut plan = plan();
    let admission = service_operation_policy_admission(&plan, "operator-a");
    plan.operations.push(HostOperation::ServiceStart {
        unit: "unadmitted.service".to_string(),
    });

    let blockers = admit_host_operation_plan(plan, admission, authorization())
        .expect_err("tampered plan must fail");
    assert!(blockers.contains(&"plan_digest_mismatch".to_string()));
}

#[test]
fn unknown_deny_missing_apply_and_wrong_authority_fail_closed() {
    for decision in [
        PolicyDecision::Unknown {
            missing_evidence: vec!["registry".to_string()],
        },
        PolicyDecision::Deny {
            violations: Vec::new(),
        },
    ] {
        let plan = plan();
        let mut admission = service_operation_policy_admission(&plan, "operator-a");
        admission.decision = decision;
        assert!(admit_host_operation_plan(plan, admission, authorization()).is_err());
    }

    let plan = plan();
    let admission = service_operation_policy_admission(&plan, "operator-a");
    let mut missing_apply = authorization();
    missing_apply.explicit_apply = false;
    assert!(admit_host_operation_plan(plan.clone(), admission.clone(), missing_apply).is_err());
    let mut wrong_authority = authorization();
    wrong_authority.authority = PolicyAuthority::RuntimePrincipal;
    assert!(admit_host_operation_plan(plan, admission, wrong_authority).is_err());
}
