use super::{
    HOST_ACTIVATION_PLAN_SCHEMA, HOST_APPLY_EXECUTION_SCHEMA, HOST_APPLY_REPORT_SCHEMA,
    HOST_BUNDLE_SCHEMA, HOST_OBSERVATION_SCHEMA, HostCommandContext, build_host_activation_plan,
    build_host_activation_plan_with_context, build_host_apply_report,
    build_host_apply_report_with_context, build_host_bundle, collect_contracts,
    execute_admitted_service_apply, execute_host_apply, execute_runtime_service_reconcile,
    execute_service_apply_with_admission, observe_host, service_apply_satisfied_preconditions,
};
use crate::command_surface::{HostCommandInvocation, HostCommandSurface, render_command};
use crate::domain::filesystem::FilesystemPolicy;
use crate::domain::packages::PackagePolicy;
use crate::domain::packages::{PackageCommandContext, PackageObservedState};
use crate::domain::principal::PrincipalPolicy;
use crate::domain::principal::{PrincipalCommandContext, PrincipalObservedState};
use crate::domain::privilege::PrivilegePolicy;
use crate::domain::privilege::{PrivilegeCommandContext, PrivilegeObservedState};
use crate::domain::services::{
    ServiceCommandContext, ServiceManagerKind, ServiceObservedState, ServiceObservedUnit,
    ServicePolicy, ServiceUnitPolicy,
};
use crate::domain::shared::{SubstrateBundleFile, SubstrateDomainContract, SurfaceStatus};
use crate::execution::AdmittedHostOperation;
use crate::ports::{
    HostCommandResult, HostCommandRunnerPort, HostContractPort, HostFilesystemPort, HostOperation,
    HostPackageObservationPort, HostPackagePort, HostPrincipalObservationPort, HostPrincipalPort,
    HostPrivilegeObservationPort, HostPrivilegePort, HostServiceObservationPort, HostServicePort,
};
use crate::runtime_authority::default_runtime_principal_layout;
use crate::runtime_layout::current_runtime_layout;
use cynic_protocol::{ExecutionAuthorization, PolicyAuthority, PolicyDecision};
use std::cell::Cell;

struct StaticHostContractPort;
struct FakeCommandRunner;
struct FakeObservationAdapter;
struct ReadyForServiceObservation;
struct ActiveServiceObservation;
struct RecordingCommandRunner {
    calls: Cell<usize>,
}

fn portable_command_context() -> HostCommandContext {
    HostCommandContext {
        principal: PrincipalCommandContext {
            getent_bin: "/portable/bin/getent".to_string(),
        },
        privilege: PrivilegeCommandContext {
            sudo_bin: "/portable/bin/sudo".to_string(),
            route_bin: "/portable/bin/ip".to_string(),
            service_manager_bin: "/portable/bin/systemctl".to_string(),
            denied_shell_bin: "/portable/bin/sh".to_string(),
        },
        packages: PackageCommandContext {
            package_query_bin: "/portable/bin/dpkg".to_string(),
            package_install_bin: "/portable/bin/apt-get".to_string(),
            apk_bin: "/portable/bin/apk".to_string(),
            dnf_bin: "/portable/bin/dnf".to_string(),
            rpm_bin: "/portable/bin/rpm".to_string(),
            which_bin: "/portable/bin/which".to_string(),
            os_release_path: "/portable/etc/os-release".to_string(),
        },
        services: ServiceCommandContext {
            systemctl_bin: "/portable/bin/systemctl".to_string(),
            rc_service_bin: "/portable/bin/rc-service".to_string(),
            rc_update_bin: "/portable/bin/rc-update".to_string(),
        },
    }
}

impl HostPrincipalPort for StaticHostContractPort {
    fn desired_principal(&self) -> PrincipalPolicy {
        let layout = current_runtime_layout();
        let principal = default_runtime_principal_layout();
        let config_dir = layout.config_dir.clone();
        let state_dirs = layout.state_dirs();
        PrincipalPolicy {
            runtime_user: principal.runtime_user,
            runtime_group: principal.runtime_group,
            home_dir: principal.home_dir.display().to_string(),
            config_dir,
            state_dirs,
        }
    }
}

impl HostPrivilegePort for StaticHostContractPort {
    fn desired_privilege(&self) -> PrivilegePolicy {
        let layout = current_runtime_layout();
        PrivilegePolicy {
            sudoers_subject: "cynic".to_string(),
            helper_dir: layout.helper_dir.clone(),
            config_dir: layout.config_dir.clone(),
            sudoers_dir: layout.sudoers_dir.clone(),
            denial_proof_required: true,
            dedicated_helpers: vec![
                "cynic-network-actuator".to_string(),
                "cynic-service-actuator".to_string(),
            ],
        }
    }
}

impl HostPackagePort for StaticHostContractPort {
    fn desired_packages(&self) -> PackagePolicy {
        let layout = current_runtime_layout();
        PackagePolicy {
            manager: crate::runtime_layout::PackageManagerKind::Apt,
            policy_dir: layout.config_dir.clone(),
            source_projection_path: format!("{}/package-sources.json", layout.config_dir),
            approved_sources: vec!["distribution_package".to_string()],
            provenance: crate::domain::package_provenance::PackageProvenancePolicy::Repository {
                source_id: "distribution_package".to_string(),
                trusted_key_fingerprint: "test-key".to_string(),
                max_metadata_age_secs: 86_400,
            },
            promotion_policy:
                crate::domain::package_provenance::PackagePromotionPolicy::critical_infrastructure(),
            bootstrap_packages: vec!["wireguard-tools".to_string(), "iproute2".to_string()],
        }
    }
}

impl HostFilesystemPort for StaticHostContractPort {
    fn desired_filesystem(&self) -> FilesystemPolicy {
        let layout = current_runtime_layout();
        FilesystemPolicy {
            managed_paths: vec![SubstrateBundleFile {
                logical_name: "state-dir".to_string(),
                install_path: layout.state_dir,
                mode: "0755".to_string(),
            }],
        }
    }
}

impl HostServicePort for StaticHostContractPort {
    fn desired_services(&self) -> ServicePolicy {
        let layout = current_runtime_layout();
        ServicePolicy {
            manager: ServiceManagerKind::Systemd,
            unit_dir: layout.service_unit_dir,
            units: vec![ServiceUnitPolicy {
                service_id: "cynic-node".to_string(),
                unit_name: "cynic-node.service".to_string(),
                enable_on_boot: true,
                restart_mode: "on-failure".to_string(),
                source: "test:static_host_contract".to_string(),
                health_port: None,
                process_name: Some("cynic-node".to_string()),
                supervisor_provider: None,
                health_kind: None,
            }],
        }
    }
}

impl HostContractPort for StaticHostContractPort {
    fn declared_contracts(&self) -> Vec<SubstrateDomainContract> {
        vec![
            self.desired_principal().contract(),
            self.desired_privilege().contract(),
            self.desired_packages().contract(),
            self.desired_filesystem().contract(),
            self.desired_services().contract(),
        ]
    }
}

impl HostCommandRunnerPort for FakeCommandRunner {
    fn run_operation(&self, admitted: &AdmittedHostOperation) -> HostCommandResult {
        let operation = admitted.operation();
        let command = match operation {
            HostOperation::EnsureSystemGroup { group } => {
                format!("ensure_system_group group={group}")
            }
            HostOperation::EnsureSystemUser { user, .. } => {
                format!("ensure_system_user user={user}")
            }
            HostOperation::EnsureSupplementaryGroupMember { user, group } => {
                format!("ensure_supplementary_group_member user={user} group={group}")
            }
            HostOperation::EnsureOwnedDir { path, .. } => {
                format!("ensure_owned_dir path={path}")
            }
            HostOperation::InstallOwnedFile { target, .. } => {
                format!("install_owned_file target={target}")
            }
            HostOperation::ValidatePrivilegeProjection { path, .. } => {
                format!("validate_privilege_projection path={path}")
            }
            HostOperation::EnsureDir { path, mode } => {
                format!("ensure_dir path={path} mode={mode:o}")
            }
            HostOperation::WritePackageSourceProjection { path, .. } => {
                format!("write_package_source_projection path={path}")
            }
            HostOperation::InstallBootstrapPackages { packages, .. } => {
                format!("install_bootstrap_packages packages={}", packages.join(","))
            }
            HostOperation::ServiceManagerDaemonReload => "systemctl daemon-reload".to_string(),
            HostOperation::ServiceEnableNow { unit } => {
                format!("systemctl enable --now {unit}")
            }
            HostOperation::ServiceStart { unit } => {
                format!("systemctl start {unit}")
            }
        };
        HostCommandResult {
            command: command.clone(),
            success: !command.contains("install_bootstrap_packages"),
            exit_code: Some(if command.contains("install_bootstrap_packages") {
                1
            } else {
                0
            }),
            stdout: String::new(),
            stderr: if command.contains("install_bootstrap_packages") {
                "package source unresolved".to_string()
            } else {
                String::new()
            },
        }
    }
}

impl HostCommandRunnerPort for RecordingCommandRunner {
    fn run_operation(&self, admitted: &AdmittedHostOperation) -> HostCommandResult {
        self.calls.set(self.calls.get() + 1);
        HostCommandResult {
            command: format!("admitted:{:?}", admitted.operation()),
            success: true,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        }
    }
}

impl HostPrincipalObservationPort for FakeObservationAdapter {
    fn observe_principal(&self, policy: &PrincipalPolicy) -> PrincipalObservedState {
        PrincipalObservedState {
            user_present: true,
            group_present: true,
            observed_home_dir: Some(policy.home_dir.clone()),
            present_state_dirs: policy.state_dirs.clone(),
            owned_paths: policy.managed_paths(),
            ownership_unknown_paths: Vec::new(),
        }
    }
}

impl HostPrivilegeObservationPort for FakeObservationAdapter {
    fn observe_privilege(&self, _policy: &PrivilegePolicy) -> PrivilegeObservedState {
        PrivilegeObservedState {
            helper_dir_present: true,
            sudoers_projection_present: true,
            denial_proof_ready: false,
        }
    }
}

impl HostPackageObservationPort for FakeObservationAdapter {
    fn observe_packages(&self, _policy: &PackagePolicy) -> PackageObservedState {
        PackageObservedState {
            installed_packages: vec!["wireguard-tools".to_string()],
            ..Default::default()
        }
    }
}

impl HostServiceObservationPort for FakeObservationAdapter {
    fn observe_services(&self, _policy: &ServicePolicy) -> ServiceObservedState {
        ServiceObservedState {
            units: vec![ServiceObservedUnit {
                unit_name: "cynic-node.service".to_string(),
                present: true,
                enabled: false,
                active: false,
            }],
        }
    }
}

impl HostPrincipalObservationPort for ReadyForServiceObservation {
    fn observe_principal(&self, policy: &PrincipalPolicy) -> PrincipalObservedState {
        PrincipalObservedState {
            user_present: true,
            group_present: true,
            observed_home_dir: Some(policy.home_dir.clone()),
            present_state_dirs: policy.state_dirs.clone(),
            owned_paths: policy.managed_paths(),
            ownership_unknown_paths: Vec::new(),
        }
    }
}

impl HostPrivilegeObservationPort for ReadyForServiceObservation {
    fn observe_privilege(&self, _policy: &PrivilegePolicy) -> PrivilegeObservedState {
        PrivilegeObservedState {
            helper_dir_present: true,
            sudoers_projection_present: true,
            denial_proof_ready: true,
        }
    }
}

impl HostPackageObservationPort for ReadyForServiceObservation {
    fn observe_packages(&self, policy: &PackagePolicy) -> PackageObservedState {
        PackageObservedState {
            installed_packages: policy.bootstrap_packages.clone(),
            ..Default::default()
        }
    }
}

impl HostServiceObservationPort for ReadyForServiceObservation {
    fn observe_services(&self, policy: &ServicePolicy) -> ServiceObservedState {
        ServiceObservedState {
            units: policy
                .units
                .iter()
                .map(|unit| ServiceObservedUnit {
                    unit_name: unit.unit_name.clone(),
                    present: true,
                    enabled: false,
                    active: false,
                })
                .collect(),
        }
    }
}

impl HostServiceObservationPort for ActiveServiceObservation {
    fn observe_services(&self, policy: &ServicePolicy) -> ServiceObservedState {
        ServiceObservedState {
            units: policy
                .units
                .iter()
                .map(|unit| ServiceObservedUnit {
                    unit_name: unit.unit_name.clone(),
                    present: true,
                    enabled: true,
                    active: true,
                })
                .collect(),
        }
    }
}

#[test]
fn collects_host_substrate_contracts() {
    let contracts = collect_contracts(&StaticHostContractPort);
    assert_eq!(contracts.len(), 5);
}

#[test]
fn principal_status_is_active_when_observed_state_matches_policy() {
    let policy = StaticHostContractPort.desired_principal();
    let layout = current_runtime_layout();
    let present_state_dirs = layout.state_dirs();
    let report = policy.status(&PrincipalObservedState {
        user_present: true,
        group_present: true,
        observed_home_dir: Some(policy.home_dir.clone()),
        present_state_dirs,
        owned_paths: policy.managed_paths(),
        ownership_unknown_paths: Vec::new(),
    });
    assert_eq!(report.status, SurfaceStatus::Active);
}

#[test]
fn privilege_status_requires_denial_proof_when_configured() {
    let policy = StaticHostContractPort.desired_privilege();
    let report = policy.status(&PrivilegeObservedState {
        helper_dir_present: true,
        sudoers_projection_present: true,
        denial_proof_ready: false,
    });
    assert_eq!(report.status, SurfaceStatus::Blocked);
}

#[test]
fn privilege_activation_plan_contains_denial_checks() {
    let policy = StaticHostContractPort.desired_privilege();
    let plan = policy.activation_plan();
    let runtime_layout = current_runtime_layout();
    let expected = render_command(&HostCommandSurface {
        role: "deny_route".to_string(),
        invocation: HostCommandInvocation::Argv {
            argv: vec![
                runtime_layout.sudo_bin(),
                "-n".to_string(),
                runtime_layout.capabilities().route_tool.command,
                "route".to_string(),
                "show".to_string(),
            ],
        },
    });
    assert!(
        plan.denial_checks
            .iter()
            .map(render_command)
            .any(|item| item == expected)
    );
}

#[test]
fn package_status_is_blocked_when_bootstrap_package_missing() {
    let policy = StaticHostContractPort.desired_packages();
    let report = policy.status(&PackageObservedState {
        installed_packages: vec!["wireguard-tools".to_string()],
        ..Default::default()
    });
    assert_eq!(report.status, SurfaceStatus::Blocked);
    assert!(report.missing_packages.contains(&"iproute2".to_string()));
}

#[test]
fn package_activation_plan_refers_to_approved_source_and_postflight_checks() {
    let policy = StaticHostContractPort.desired_packages();
    let plan = policy.activation_plan();
    assert!(
        plan.preflight
            .iter()
            .map(render_command)
            .any(|item| item.contains("approved package source"))
    );
    assert!(
        plan.postflight
            .iter()
            .map(render_command)
            .any(|item| item.contains("wireguard-tools"))
    );
}

#[test]
fn service_status_is_blocked_when_unit_disabled_or_inactive() {
    let policy = StaticHostContractPort.desired_services();
    let report = policy.status(&ServiceObservedState {
        units: vec![ServiceObservedUnit {
            unit_name: "cynic-node.service".to_string(),
            present: true,
            enabled: false,
            active: false,
        }],
    });
    assert_eq!(report.status, SurfaceStatus::Blocked);
    assert!(
        report
            .disabled_units
            .contains(&"cynic-node.service".to_string())
    );
    assert!(
        report
            .inactive_units
            .contains(&"cynic-node.service".to_string())
    );
}

#[test]
fn service_bundle_projects_systemd_unit_paths() {
    let policy = StaticHostContractPort.desired_services();
    let bundle = policy.bundle();
    let runtime_layout = current_runtime_layout();
    let expected_unit_path = format!("{}/cynic-node.service", runtime_layout.service_unit_dir);
    assert!(
        bundle
            .files
            .iter()
            .any(|item| item.install_path == expected_unit_path)
    );
}

#[test]
fn local_observer_returns_four_domain_reports() {
    let reports = observe_host(&StaticHostContractPort, &FakeObservationAdapter);
    assert_eq!(reports.schema, HOST_OBSERVATION_SCHEMA);
    assert_eq!(reports.status, SurfaceStatus::Blocked);
    let first_blocked = reports
        .blocked_domains
        .first()
        .copied()
        .expect("expected at least one blocked host domain");
    match first_blocked {
        "principal" => {
            assert_eq!(reports.lifecycle_status, reports.principal.lifecycle_status);
            assert_eq!(
                reports.lifecycle_disposition,
                reports.principal.lifecycle_disposition
            );
            assert_eq!(
                reports.next_boundary_kind,
                reports.principal.next_boundary_kind
            );
            assert_eq!(reports.next_command, reports.principal.next_command);
        }
        "privilege" => {
            assert_eq!(reports.lifecycle_status, reports.privilege.lifecycle_status);
            assert_eq!(
                reports.lifecycle_disposition,
                reports.privilege.lifecycle_disposition
            );
            assert_eq!(
                reports.next_boundary_kind,
                reports.privilege.next_boundary_kind
            );
            assert_eq!(reports.next_command, reports.privilege.next_command);
        }
        "packages" => {
            assert_eq!(reports.lifecycle_status, reports.packages.lifecycle_status);
            assert_eq!(
                reports.lifecycle_disposition,
                reports.packages.lifecycle_disposition
            );
            assert_eq!(
                reports.next_boundary_kind,
                reports.packages.next_boundary_kind
            );
            assert_eq!(reports.next_command, reports.packages.next_command);
        }
        "services" => {
            assert_eq!(reports.lifecycle_status, reports.services.lifecycle_status);
            assert_eq!(
                reports.lifecycle_disposition,
                reports.services.lifecycle_disposition
            );
            assert_eq!(
                reports.next_boundary_kind,
                reports.services.next_boundary_kind
            );
            assert_eq!(reports.next_command, reports.services.next_command);
        }
        other => panic!("unexpected blocked domain: {other}"),
    }
    assert!(!reports.blocked_domains.is_empty());
    assert!(!reports.principal.evidences.is_empty());
    assert!(!reports.privilege.evidences.is_empty());
    assert!(!reports.packages.evidences.is_empty());
    assert!(!reports.services.evidences.is_empty());
}

#[test]
fn bundle_report_aggregates_domain_bundles() {
    let bundle = build_host_bundle(&StaticHostContractPort);
    let runtime_layout = current_runtime_layout();
    let expected_unit_path = format!("{}/cynic-node.service", runtime_layout.service_unit_dir);
    assert_eq!(bundle.schema, HOST_BUNDLE_SCHEMA);
    assert_eq!(bundle.status, SurfaceStatus::Planned);
    assert!(
        bundle
            .services
            .files
            .iter()
            .any(|item| item.install_path == expected_unit_path)
    );
}

#[test]
fn activation_plan_marks_blocked_domains_and_next_actions() {
    let observed = observe_host(&StaticHostContractPort, &FakeObservationAdapter);
    let plan = build_host_activation_plan(&StaticHostContractPort, &observed);
    assert_eq!(plan.schema, HOST_ACTIVATION_PLAN_SCHEMA);
    assert_eq!(plan.status, SurfaceStatus::Blocked);
    assert!(!plan.blocked_domains.is_empty());
    assert!(!plan.next_actions.is_empty());
}

#[test]
fn activation_plan_uses_injected_host_command_context() {
    let observed = observe_host(&StaticHostContractPort, &FakeObservationAdapter);
    let plan = build_host_activation_plan_with_context(
        &StaticHostContractPort,
        &observed,
        &portable_command_context(),
    );
    let rendered_commands = plan
        .principal
        .preflight
        .iter()
        .chain(plan.privilege.denial_checks.iter())
        .chain(plan.packages.preflight.iter())
        .chain(plan.packages.install_commands.iter())
        .chain(plan.packages.postflight.iter())
        .chain(plan.services.install_commands.iter())
        .chain(plan.services.postflight.iter())
        .map(render_command)
        .collect::<Vec<_>>();

    for expected in [
        "/portable/bin/getent passwd cynic",
        "/portable/bin/sudo -n /portable/bin/ip route show",
        "/portable/bin/apt-get install -y wireguard-tools iproute2",
        "/portable/bin/dpkg -s wireguard-tools",
        "/portable/bin/systemctl enable --now cynic-node.service",
        "test -f /portable/etc/os-release",
    ] {
        assert!(
            rendered_commands.iter().any(|item| item.contains(expected)),
            "expected portable command fragment not found: {expected}"
        );
    }
}

#[test]
fn apply_report_propagates_dry_run_and_blockers() {
    let observed = observe_host(&StaticHostContractPort, &FakeObservationAdapter);
    let report = build_host_apply_report(&StaticHostContractPort, &observed, true);
    assert_eq!(report.schema, HOST_APPLY_REPORT_SCHEMA);
    assert!(report.dry_run);
    assert_eq!(report.status, SurfaceStatus::Blocked);
    assert!(!report.blocked_domains.is_empty());
}

#[test]
fn apply_report_uses_injected_host_command_context() {
    let observed = observe_host(&StaticHostContractPort, &FakeObservationAdapter);
    let report = build_host_apply_report_with_context(
        &StaticHostContractPort,
        &observed,
        true,
        &portable_command_context(),
    );
    let rendered_commands = report
        .packages
        .commands
        .iter()
        .chain(report.services.commands.iter())
        .map(render_command)
        .collect::<Vec<_>>();

    assert!(report.dry_run);
    assert!(
        report
            .principal
            .proofs_required
            .iter()
            .any(|item| item == "/portable/bin/getent passwd cynic")
    );
    assert!(
        report
            .privilege
            .denial_proofs_required
            .iter()
            .any(|item| item.contains("/portable/bin/sudo -n /portable/bin/ip route show"))
    );
    assert!(
        rendered_commands
            .iter()
            .any(|item| item.contains("/portable/bin/apt-get install -y wireguard-tools iproute2"))
    );
    assert!(
        rendered_commands
            .iter()
            .any(|item| item.contains("/portable/bin/systemctl enable --now cynic-node.service"))
    );
}

#[test]
fn execute_host_apply_in_dry_run_emits_commands_without_running_them() {
    let observed = observe_host(&StaticHostContractPort, &FakeObservationAdapter);
    let report = execute_host_apply(&StaticHostContractPort, &observed, true, &FakeCommandRunner);
    assert_eq!(report.schema, HOST_APPLY_EXECUTION_SCHEMA);
    assert!(report.dry_run);
    assert!(report.commands.iter().all(|item| !item.executed));
}

#[test]
fn legacy_host_apply_cannot_reach_runner_when_mutating() {
    let observed = observe_host(&StaticHostContractPort, &FakeObservationAdapter);
    let report = execute_host_apply(
        &StaticHostContractPort,
        &observed,
        false,
        &FakeCommandRunner,
    );
    assert!(report.commands.iter().all(|item| !item.executed));
    assert_eq!(report.status, SurfaceStatus::Blocked);
    assert_eq!(report.blockers, vec!["policy_admission_required"]);
    assert!(report.admission.is_none());
}

#[test]
fn admitted_service_apply_runs_exact_plan_and_requires_postflight_proofs() {
    let observed = observe_host(&StaticHostContractPort, &ReadyForServiceObservation);
    let runner = RecordingCommandRunner {
        calls: Cell::new(0),
    };
    let authorization = ExecutionAuthorization::new(
        "operator-a",
        PolicyAuthority::HumanOperator,
        true,
        service_apply_satisfied_preconditions(&observed),
    );

    let report = execute_admitted_service_apply(
        &StaticHostContractPort,
        &observed,
        authorization,
        &runner,
        &ActiveServiceObservation,
    );

    assert_eq!(runner.calls.get(), 3);
    assert_eq!(report.status, SurfaceStatus::Active);
    assert!(report.blockers.is_empty());
    assert_eq!(report.proofs_observed, report.proofs_required);
    assert!(report.commands.iter().all(|command| command.executed));
}

#[test]
fn runtime_service_reconcile_requires_runtime_authority_and_exact_policy() {
    let observed = observe_host(&StaticHostContractPort, &ReadyForServiceObservation);
    let runner = RecordingCommandRunner {
        calls: Cell::new(0),
    };
    let authorization = ExecutionAuthorization::new(
        "runtime-host-helper",
        PolicyAuthority::RuntimeAuthority,
        true,
        service_apply_satisfied_preconditions(&observed),
    );

    let report = execute_runtime_service_reconcile(
        &StaticHostContractPort,
        &observed,
        authorization,
        &runner,
        &ActiveServiceObservation,
    );

    assert_eq!(runner.calls.get(), 3);
    assert_eq!(report.status, SurfaceStatus::Active);
    let admission = report.admission.expect("runtime admission");
    assert_eq!(
        admission.policy_set.id,
        "cynic.host.services.runtime_reconcile"
    );
    assert_eq!(
        admission.mutation.required_authority,
        PolicyAuthority::RuntimeAuthority
    );
}

#[test]
fn operator_principal_or_missing_apply_cannot_use_runtime_reconcile() {
    let observed = observe_host(&StaticHostContractPort, &ReadyForServiceObservation);
    for (authority, apply) in [
        (PolicyAuthority::HumanOperator, true),
        (PolicyAuthority::HumanAdmin, true),
        (PolicyAuthority::RuntimePrincipal, true),
        (PolicyAuthority::RuntimeAuthority, false),
    ] {
        let runner = RecordingCommandRunner {
            calls: Cell::new(0),
        };
        let authorization = ExecutionAuthorization::new(
            "runtime-host-helper",
            authority,
            apply,
            service_apply_satisfied_preconditions(&observed),
        );

        let report = execute_runtime_service_reconcile(
            &StaticHostContractPort,
            &observed,
            authorization,
            &runner,
            &ActiveServiceObservation,
        );

        assert_eq!(runner.calls.get(), 0);
        assert_eq!(report.status, SurfaceStatus::Blocked);
    }
}

#[test]
fn successful_mutation_without_postflight_proof_remains_blocked() {
    let observed = observe_host(&StaticHostContractPort, &ReadyForServiceObservation);
    let runner = RecordingCommandRunner {
        calls: Cell::new(0),
    };

    let report = execute_admitted_service_apply(
        &StaticHostContractPort,
        &observed,
        valid_service_authorization(&observed),
        &runner,
        &ReadyForServiceObservation,
    );

    assert_eq!(runner.calls.get(), 3);
    assert_eq!(report.status, SurfaceStatus::Blocked);
    assert!(
        report
            .blockers
            .iter()
            .any(|blocker| blocker.starts_with("postflight_proof_missing:"))
    );
}

#[test]
fn denied_unknown_tampered_or_under_authorized_plan_never_reaches_runner() {
    let observed = observe_host(&StaticHostContractPort, &ReadyForServiceObservation);
    let policy = StaticHostContractPort.desired_services();
    let base_plan = crate::execution::build_service_operation_plan(&policy, "local-host");
    let base_admission =
        crate::execution::service_operation_policy_admission(&base_plan, "operator-a");

    for decision in [
        PolicyDecision::Deny {
            violations: Vec::new(),
        },
        PolicyDecision::Unknown {
            missing_evidence: vec!["runtime_registry".to_string()],
        },
    ] {
        let runner = RecordingCommandRunner {
            calls: Cell::new(0),
        };
        let mut admission = base_admission.clone();
        admission.decision = decision;
        let report = execute_service_apply_with_admission(
            &observed,
            &policy,
            base_plan.clone(),
            admission,
            valid_service_authorization(&observed),
            &runner,
            &ActiveServiceObservation,
        );
        assert_eq!(runner.calls.get(), 0);
        assert_eq!(report.status, SurfaceStatus::Blocked);
    }

    let runner = RecordingCommandRunner {
        calls: Cell::new(0),
    };
    let mut tampered = base_plan.clone();
    tampered.operations.push(HostOperation::ServiceStart {
        unit: "unadmitted.service".to_string(),
    });
    let report = execute_service_apply_with_admission(
        &observed,
        &policy,
        tampered,
        base_admission.clone(),
        valid_service_authorization(&observed),
        &runner,
        &ActiveServiceObservation,
    );
    assert_eq!(runner.calls.get(), 0);
    assert!(
        report
            .blockers
            .contains(&"plan_not_derived_from_desired_service_policy".to_string())
    );

    let runner = RecordingCommandRunner {
        calls: Cell::new(0),
    };
    let mut weak = valid_service_authorization(&observed);
    weak.authority = PolicyAuthority::RuntimePrincipal;
    let report = execute_service_apply_with_admission(
        &observed,
        &policy,
        base_plan,
        base_admission,
        weak,
        &runner,
        &ActiveServiceObservation,
    );
    assert_eq!(runner.calls.get(), 0);
    assert!(
        report
            .blockers
            .contains(&"authority_insufficient".to_string())
    );

    let runner = RecordingCommandRunner {
        calls: Cell::new(0),
    };
    let mut no_apply = valid_service_authorization(&observed);
    no_apply.explicit_apply = false;
    let plan = crate::execution::build_service_operation_plan(&policy, "local-host");
    let admission = crate::execution::service_operation_policy_admission(&plan, "operator-a");
    let report = execute_service_apply_with_admission(
        &observed,
        &policy,
        plan,
        admission,
        no_apply,
        &runner,
        &ActiveServiceObservation,
    );
    assert_eq!(runner.calls.get(), 0);
    assert!(
        report
            .blockers
            .contains(&"explicit_apply_required".to_string())
    );
}

fn valid_service_authorization(observed: &super::HostObservationReports) -> ExecutionAuthorization {
    ExecutionAuthorization::new(
        "operator-a",
        PolicyAuthority::HumanOperator,
        true,
        service_apply_satisfied_preconditions(observed),
    )
}
