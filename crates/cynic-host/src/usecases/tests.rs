use super::{
    HOST_ACTIVATION_PLAN_SCHEMA, HOST_APPLY_EXECUTION_SCHEMA, HOST_APPLY_REPORT_SCHEMA,
    HOST_BUNDLE_SCHEMA, HOST_OBSERVATION_SCHEMA, HostCommandContext, build_host_activation_plan,
    build_host_activation_plan_with_context, build_host_apply_report,
    build_host_apply_report_with_context, build_host_bundle, collect_contracts, execute_host_apply,
    observe_host,
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
use crate::ports::{
    HostCommandResult, HostCommandRunnerPort, HostContractPort, HostFilesystemPort, HostOperation,
    HostPackageObservationPort, HostPackagePort, HostPrincipalObservationPort, HostPrincipalPort,
    HostPrivilegeObservationPort, HostPrivilegePort, HostServiceObservationPort, HostServicePort,
};
use crate::runtime_layout::current_runtime_layout;

struct StaticHostContractPort;
struct FakeCommandRunner;
struct FakeObservationAdapter;

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
        let config_dir = layout.config_dir.clone();
        let state_dirs = layout.state_dirs();
        PrincipalPolicy {
            runtime_user: "cynic".to_string(),
            runtime_group: "cynic".to_string(),
            home_dir: layout.runtime_home_dir,
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
    fn run_operation(&self, operation: &HostOperation) -> HostCommandResult {
        let command = match operation {
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

impl HostPrincipalObservationPort for FakeObservationAdapter {
    fn observe_principal(&self, policy: &PrincipalPolicy) -> PrincipalObservedState {
        PrincipalObservedState {
            user_present: true,
            group_present: true,
            observed_home_dir: Some(policy.home_dir.clone()),
            present_state_dirs: policy.state_dirs.clone(),
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
        observed_home_dir: Some(layout.runtime_home_dir),
        present_state_dirs,
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
fn execute_host_apply_reports_runner_failures_when_mutating() {
    let observed = observe_host(&StaticHostContractPort, &FakeObservationAdapter);
    let report = execute_host_apply(
        &StaticHostContractPort,
        &observed,
        false,
        &FakeCommandRunner,
    );
    assert!(report.commands.iter().any(|item| item.executed));
    assert_eq!(report.status, SurfaceStatus::Blocked);
    assert!(report.commands.iter().any(|item| {
        item.result
            .as_ref()
            .is_some_and(|result| !result.success && result.stderr == "package source unresolved")
    }));
}
