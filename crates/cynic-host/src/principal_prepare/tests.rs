use std::cell::Cell;

use cynic_protocol::{ExecutionAuthorization, PolicyAuthority};

use super::{
    PRINCIPAL_PREPARE_EXECUTION_SCHEMA, PRINCIPAL_PREPARE_PLAN_SCHEMA, principal_prepare_apply,
    principal_prepare_plan, principal_prepare_plan_with_resolution,
    principal_prepare_preconditions,
};
use crate::domain::principal::{PrincipalObservedState, PrincipalPolicy};
use crate::execution::AdmittedHostOperation;
use crate::ports::{
    HostCommandResult, HostCommandRunnerPort, HostOperation, HostPrincipalObservationPort,
};
use crate::registry::HostProfileResolution;

struct RecordingRunner {
    calls: Cell<usize>,
}

struct ReadyObserver;
struct MisownedObserver;

impl HostCommandRunnerPort for RecordingRunner {
    fn run_operation(&self, operation: &AdmittedHostOperation) -> HostCommandResult {
        self.calls.set(self.calls.get() + 1);
        HostCommandResult {
            command: format!("admitted:{:?}", operation.operation()),
            success: true,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        }
    }
}

impl HostPrincipalObservationPort for ReadyObserver {
    fn observe_principal(&self, policy: &PrincipalPolicy) -> PrincipalObservedState {
        PrincipalObservedState {
            user_present: true,
            group_present: true,
            observed_home_dir: Some(policy.home_dir.clone()),
            present_state_dirs: policy.state_dirs.clone(),
            owned_paths: policy.managed_paths(),
<<<<<<< HEAD
            ownership_unknown_paths: Vec::new(),
=======
>>>>>>> github/main
        }
    }
}

impl HostPrincipalObservationPort for MisownedObserver {
    fn observe_principal(&self, policy: &PrincipalPolicy) -> PrincipalObservedState {
        PrincipalObservedState {
            user_present: true,
            group_present: true,
            observed_home_dir: Some(policy.home_dir.clone()),
            present_state_dirs: policy.state_dirs.clone(),
            owned_paths: Vec::new(),
<<<<<<< HEAD
            ownership_unknown_paths: Vec::new(),
=======
>>>>>>> github/main
        }
    }
}

fn policy() -> PrincipalPolicy {
    PrincipalPolicy {
        runtime_user: "cynic".to_string(),
        runtime_group: "cynic".to_string(),
        home_dir: "/var/lib/cynic".to_string(),
        config_dir: "/etc/cynic".to_string(),
        state_dirs: vec!["/var/log/cynic".to_string()],
    }
}

fn authorization(authority: PolicyAuthority, explicit_apply: bool) -> ExecutionAuthorization {
    ExecutionAuthorization::new(
        "admin-a",
        authority,
        explicit_apply,
        principal_prepare_preconditions(),
    )
}

#[test]
fn plan_contains_only_semantic_provider_agnostic_operations() {
    let report = principal_prepare_plan(&policy());
    let json = serde_json::to_string(&report).expect("serialize plan");

    assert_eq!(report.schema, PRINCIPAL_PREPARE_PLAN_SCHEMA);
    assert_eq!(report.status, "planned");
    assert!(!report.mutates);
    assert!(
        report
            .operation_plan
            .operations
            .iter()
            .any(|operation| matches!(operation, HostOperation::EnsureSystemUser { .. }))
    );
    assert!(!json.contains("useradd"));
    assert!(!json.contains("adduser"));
    assert!(!json.contains("/usr/sbin/nologin"));
}

#[test]
fn plan_uses_resolved_target_when_available() {
    let resolution = HostProfileResolution {
        hostname_observed: "bootstrap-host".to_string(),
        matched_node: Some("cynic-cortex".to_string()),
        matched_role: None,
        canonical_role: None,
        profile_applied: "test-profile",
        match_source: "node_name",
        host_bootstrap_target_stage: None,
        host_bootstrap_required_domains: Vec::new(),
        registry_path: None,
    };

    let report = principal_prepare_plan_with_resolution(&policy(), Some(&resolution));

    assert_eq!(report.operation_plan.target, "cynic-cortex");
    assert_eq!(
        report.operation_plan.id,
        "host.principal.prepare:cynic-cortex"
    );
}

#[test]
fn human_admin_apply_executes_and_requires_all_postflight_proofs() {
    let runner = RecordingRunner {
        calls: Cell::new(0),
    };
    let report = principal_prepare_apply(
        &policy(),
        None,
        authorization(PolicyAuthority::HumanAdmin, true),
        &runner,
        &ReadyObserver,
    );

    assert_eq!(report.schema, PRINCIPAL_PREPARE_EXECUTION_SCHEMA);
    assert_eq!(report.status, "completed");
    assert_eq!(runner.calls.get(), report.operations.len());
    assert_eq!(report.proofs_observed, report.proofs_required);
}

#[test]
fn missing_apply_or_human_admin_authority_never_reaches_runner() {
    for authorization in [
        authorization(PolicyAuthority::HumanAdmin, false),
        authorization(PolicyAuthority::HumanOperator, true),
    ] {
        let runner = RecordingRunner {
            calls: Cell::new(0),
        };
        let report =
            principal_prepare_apply(&policy(), None, authorization, &runner, &ReadyObserver);
        assert_eq!(runner.calls.get(), 0);
        assert_eq!(report.status, "blocked");
    }
}

#[test]
fn successful_operations_without_ownership_proof_remain_blocked() {
    let runner = RecordingRunner {
        calls: Cell::new(0),
    };
    let report = principal_prepare_apply(
        &policy(),
        None,
        authorization(PolicyAuthority::HumanAdmin, true),
        &runner,
        &MisownedObserver,
    );

    assert!(runner.calls.get() > 0);
    assert_eq!(report.status, "blocked");
    assert!(
        report
            .blockers
            .iter()
            .any(|blocker| blocker.starts_with("postflight_proof_missing:path_owned:"))
    );
}
