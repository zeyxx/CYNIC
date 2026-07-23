use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

use cynic_protocol::{ExecutionAuthorization, PolicyAuthority};

use super::{
    PrivilegeBridgeInstallSpec, PrivilegeBridgeObservedState, privilege_bridge_apply,
    privilege_bridge_plan,
};
use crate::execution::AdmittedHostOperation;
use crate::ports::{
    HostCommandResult, HostCommandRunnerPort, HostOperation, HostPrivilegeBridgeObservationPort,
};

fn spec() -> PrivilegeBridgeInstallSpec {
    PrivilegeBridgeInstallSpec {
        target: "node-a".to_string(),
        bundle_complete: true,
        subject: "cynic".to_string(),
        authority_user: "root".to_string(),
        authority_group: "root".to_string(),
        provider: "sudoers".to_string(),
        helper_source: "/bundle/cynic-host-exec".to_string(),
        helper_target: "/libexec/cynic-host-exec".to_string(),
        helper_mode: 0o755,
        projection_source: "/bundle/privilege-policy".to_string(),
        projection_target: "/policy/cynic-host-exec".to_string(),
        projection_mode: 0o440,
        expected_projection: "bounded projection".to_string(),
        proof_invocation: vec![
            "/libexec/cynic-host-exec".to_string(),
            "principal-prepare".to_string(),
        ],
    }
}

fn preflight() -> PrivilegeBridgeObservedState {
    PrivilegeBridgeObservedState {
        subject_present: true,
        provider_available: true,
        ..PrivilegeBridgeObservedState::default()
    }
}

fn complete() -> PrivilegeBridgeObservedState {
    PrivilegeBridgeObservedState {
        subject_present: true,
        provider_available: true,
        helper_present: true,
        helper_executable: true,
        helper_mode_matches: true,
        helper_authority_owned: true,
        projection_present: true,
        projection_mode_matches: true,
        projection_authority_owned: true,
        projection_valid: true,
        invocation_policy_bounded: true,
        bounded_helper_noninteractive: true,
        unrestricted_shell_denied: true,
    }
}

fn authorization(authority: PolicyAuthority, apply: bool) -> ExecutionAuthorization {
    ExecutionAuthorization::new(
        "test-admin",
        authority,
        apply,
        crate::execution::privilege_bridge_preconditions(),
    )
}

struct FakeRunner {
    calls: Cell<usize>,
    success: bool,
}

impl HostCommandRunnerPort for FakeRunner {
    fn run_operation(&self, admitted: &AdmittedHostOperation) -> HostCommandResult {
        self.calls.set(self.calls.get() + 1);
        HostCommandResult {
            command: format!("{:?}", admitted.operation()),
            success: self.success,
            exit_code: Some(if self.success { 0 } else { 1 }),
            stdout: String::new(),
            stderr: String::new(),
        }
    }
}

struct FakeObserver {
    states: RefCell<VecDeque<PrivilegeBridgeObservedState>>,
}

impl FakeObserver {
    fn new(states: Vec<PrivilegeBridgeObservedState>) -> Self {
        Self {
            states: RefCell::new(states.into()),
        }
    }
}

impl HostPrivilegeBridgeObservationPort for FakeObserver {
    fn observe_privilege_bridge(
        &self,
        _spec: &PrivilegeBridgeInstallSpec,
    ) -> PrivilegeBridgeObservedState {
        self.states
            .borrow_mut()
            .pop_front()
            .expect("one observation per phase")
    }
}

#[test]
fn plan_contains_only_semantic_operations_and_injected_provider() {
    let plan = privilege_bridge_plan(&spec());
    let json = serde_json::to_string(&plan).expect("serialize plan");

    assert_eq!(plan.target, "node-a");
    assert_eq!(plan.operations.len(), 5);
    assert!(matches!(
        plan.operations.last(),
        Some(HostOperation::ValidatePrivilegeProjection { provider, .. })
            if provider == "sudoers"
    ));
    assert!(!json.contains("visudo"));
    assert!(!json.contains("sudo -n"));
    assert!(!json.contains("install -o"));
}

#[test]
fn human_admin_apply_executes_and_requires_runtime_secret_independence() {
    let runner = FakeRunner {
        calls: Cell::new(0),
        success: true,
    };
    let observer = FakeObserver::new(vec![preflight(), complete()]);

    let report = privilege_bridge_apply(
        &spec(),
        authorization(PolicyAuthority::HumanAdmin, true),
        &runner,
        &observer,
    );

    assert_eq!(report.status, "completed");
    assert_eq!(runner.calls.get(), 5);
    assert!(report.blockers.is_empty());
    assert!(
        report
            .proofs_observed
            .iter()
            .any(|proof| { proof.gate == "bounded_helper_noninteractive_and_shell_denied" })
    );
}

#[test]
fn missing_apply_or_admin_authority_executes_nothing() {
    for authorization in [
        authorization(PolicyAuthority::HumanAdmin, false),
        authorization(PolicyAuthority::HumanOperator, true),
    ] {
        let runner = FakeRunner {
            calls: Cell::new(0),
            success: true,
        };
        let observer = FakeObserver::new(Vec::new());

        let report = privilege_bridge_apply(&spec(), authorization, &runner, &observer);

        assert_eq!(report.status, "blocked");
        assert_eq!(runner.calls.get(), 0);
    }
}

#[test]
fn failed_observed_precondition_executes_nothing() {
    let runner = FakeRunner {
        calls: Cell::new(0),
        success: true,
    };
    let observer = FakeObserver::new(vec![PrivilegeBridgeObservedState {
        provider_available: true,
        ..PrivilegeBridgeObservedState::default()
    }]);

    let report = privilege_bridge_apply(
        &spec(),
        authorization(PolicyAuthority::HumanAdmin, true),
        &runner,
        &observer,
    );

    assert_eq!(report.status, "blocked");
    assert_eq!(runner.calls.get(), 0);
    assert!(
        report
            .blockers
            .contains(&"runtime_principal_missing".to_string())
    );
}

#[test]
fn successful_commands_without_denial_proof_remain_blocked() {
    let runner = FakeRunner {
        calls: Cell::new(0),
        success: true,
    };
    let mut incomplete = complete();
    incomplete.unrestricted_shell_denied = false;
    let observer = FakeObserver::new(vec![preflight(), incomplete]);

    let report = privilege_bridge_apply(
        &spec(),
        authorization(PolicyAuthority::HumanAdmin, true),
        &runner,
        &observer,
    );

    assert_eq!(report.status, "blocked");
    assert!(report.blockers.iter().any(|blocker| {
        blocker == "postflight_proof_missing:bounded_helper_noninteractive_and_shell_denied"
    }));
}
