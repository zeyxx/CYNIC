use serde::Serialize;

use crate::command_surface::{HostCommandSurface, render_command};
use crate::domain::shared::SubstrateDomainContract;
use crate::domain::shared::SurfaceStatus;
use crate::ports::{
    HostCommandResult, HostCommandRunnerPort, HostContractPort, HostOperation,
    HostPackageObservationPort, HostPrincipalObservationPort, HostPrivilegeObservationPort,
    HostServiceObservationPort,
};
use crate::registry::HostProfileResolution;

pub const HOST_OBSERVATION_SCHEMA: &str = "cynic.host_substrate.observation.v1";
pub const HOST_BUNDLE_SCHEMA: &str = "cynic.host_substrate.bundle.v1";
pub const HOST_ACTIVATION_PLAN_SCHEMA: &str = "cynic.host_substrate.activation_plan.v1";
pub const HOST_APPLY_REPORT_SCHEMA: &str = "cynic.host_substrate.apply_report.v1";
pub const HOST_APPLY_EXECUTION_SCHEMA: &str = "cynic.host_substrate.apply_execution.v1";

#[derive(Debug, Serialize)]
pub struct HostObservationReports {
    pub schema: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub status: SurfaceStatus,
    pub blocked_domains: Vec<&'static str>,
    pub principal: crate::domain::principal::PrincipalStatusReport,
    pub privilege: crate::domain::privilege::PrivilegeStatusReport,
    pub packages: crate::domain::packages::PackageStatusReport,
    pub services: crate::domain::services::ServiceStatusReport,
}

#[derive(Debug, Serialize)]
pub struct HostBundleReport {
    pub schema: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub status: SurfaceStatus,
    pub blocked_domains: Vec<&'static str>,
    pub principal: crate::domain::principal::PrincipalBundleReport,
    pub privilege: crate::domain::privilege::PrivilegeBundleReport,
    pub packages: crate::domain::packages::PackageBundleReport,
    pub services: crate::domain::services::ServiceBundleReport,
}

#[derive(Debug, Serialize)]
pub struct HostActivationPlanReport {
    pub schema: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub status: SurfaceStatus,
    pub blocked_domains: Vec<&'static str>,
    pub next_actions: Vec<&'static str>,
    pub principal: crate::domain::principal::PrincipalActivationPlan,
    pub privilege: crate::domain::privilege::PrivilegeActivationPlan,
    pub packages: crate::domain::packages::PackageActivationPlan,
    pub services: crate::domain::services::ServiceActivationPlan,
}

#[derive(Debug, Serialize)]
pub struct HostApplyReport {
    pub schema: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub status: SurfaceStatus,
    pub dry_run: bool,
    pub blocked_domains: Vec<&'static str>,
    pub next_actions: Vec<&'static str>,
    pub principal: crate::domain::principal::PrincipalApplyReport,
    pub privilege: crate::domain::privilege::PrivilegeApplyReport,
    pub packages: crate::domain::packages::PackageApplyReport,
    pub services: crate::domain::services::ServiceApplyReport,
}

#[derive(Debug, Serialize)]
pub struct HostApplyExecutionReport {
    pub schema: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub status: SurfaceStatus,
    pub dry_run: bool,
    pub blocked_domains: Vec<&'static str>,
    pub next_actions: Vec<&'static str>,
    pub commands: Vec<ExecutedHostCommand>,
}

#[derive(Debug, Serialize)]
pub struct ExecutedHostCommand {
    pub domain: &'static str,
    pub command: String,
    pub executed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ExecutedHostCommandResult>,
}

#[derive(Debug, Serialize)]
pub struct ExecutedHostCommandResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn collect_contracts(port: &impl HostContractPort) -> Vec<SubstrateDomainContract> {
    port.declared_contracts()
}

pub fn observe_host(
    desired: &impl HostContractPort,
    observed: &(
         impl HostPrincipalObservationPort
         + HostPrivilegeObservationPort
         + HostPackageObservationPort
         + HostServiceObservationPort
     ),
) -> HostObservationReports {
    observe_host_with_resolution(desired, observed, None)
}

pub fn observe_host_with_resolution(
    desired: &impl HostContractPort,
    observed: &(
         impl HostPrincipalObservationPort
         + HostPrivilegeObservationPort
         + HostPackageObservationPort
         + HostServiceObservationPort
     ),
    host_resolution: Option<HostProfileResolution>,
) -> HostObservationReports {
    let principal_policy = desired.desired_principal();
    let privilege_policy = desired.desired_privilege();
    let package_policy = desired.desired_packages();
    let service_policy = desired.desired_services();
    let principal = principal_policy.status(&observed.observe_principal(&principal_policy));
    let privilege = privilege_policy.status(&observed.observe_privilege(&privilege_policy));
    let packages = package_policy.status(&observed.observe_packages(&package_policy));
    let services = service_policy.status(&observed.observe_services(&service_policy));
    let blocked_domains = blocked_domain_statuses(
        &principal.status,
        &privilege.status,
        &packages.status,
        &services.status,
    );
    let status = if blocked_domains.is_empty() {
        SurfaceStatus::Active
    } else {
        SurfaceStatus::Blocked
    };

    HostObservationReports {
        schema: HOST_OBSERVATION_SCHEMA,
        host_resolution,
        status,
        blocked_domains,
        principal,
        privilege,
        packages,
        services,
    }
}

pub fn build_host_bundle(port: &impl HostContractPort) -> HostBundleReport {
    build_host_bundle_with_resolution(port, None)
}

pub fn build_host_bundle_with_resolution(
    port: &impl HostContractPort,
    host_resolution: Option<HostProfileResolution>,
) -> HostBundleReport {
    HostBundleReport {
        schema: HOST_BUNDLE_SCHEMA,
        host_resolution,
        status: SurfaceStatus::Planned,
        blocked_domains: Vec::new(),
        principal: port.desired_principal().bundle(),
        privilege: port.desired_privilege().bundle(),
        packages: port.desired_packages().bundle(),
        services: port.desired_services().bundle(),
    }
}

pub fn build_host_activation_plan(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
) -> HostActivationPlanReport {
    build_host_activation_plan_with_resolution(port, observed, observed.host_resolution.clone())
}

pub fn build_host_activation_plan_with_resolution(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    host_resolution: Option<HostProfileResolution>,
) -> HostActivationPlanReport {
    let blocked_domains = blocked_domains(observed);
    HostActivationPlanReport {
        schema: HOST_ACTIVATION_PLAN_SCHEMA,
        host_resolution,
        status: overall_status(observed),
        next_actions: next_actions(&blocked_domains),
        blocked_domains,
        principal: port.desired_principal().activation_plan(),
        privilege: port.desired_privilege().activation_plan(),
        packages: port.desired_packages().activation_plan(),
        services: port.desired_services().activation_plan(),
    }
}

pub fn build_host_apply_report(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    dry_run: bool,
) -> HostApplyReport {
    build_host_apply_report_with_resolution(
        port,
        observed,
        dry_run,
        observed.host_resolution.clone(),
    )
}

pub fn build_host_apply_report_with_resolution(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    dry_run: bool,
    host_resolution: Option<HostProfileResolution>,
) -> HostApplyReport {
    let blocked_domains = blocked_domains(observed);
    HostApplyReport {
        schema: HOST_APPLY_REPORT_SCHEMA,
        host_resolution,
        status: overall_status(observed),
        dry_run,
        next_actions: next_actions(&blocked_domains),
        blocked_domains,
        principal: port.desired_principal().apply_report(dry_run),
        privilege: port.desired_privilege().apply_report(dry_run),
        packages: port.desired_packages().apply_report(dry_run),
        services: port.desired_services().apply_report(dry_run),
    }
}

pub fn execute_host_apply(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    dry_run: bool,
    runner: &impl HostCommandRunnerPort,
) -> HostApplyExecutionReport {
    let apply = build_host_apply_report(port, observed, dry_run);
    let mut commands = Vec::new();
    extend_commands(
        &mut commands,
        "principal",
        &apply.principal.commands,
        &port.desired_principal().apply_operations(),
        dry_run,
        runner,
    );
    extend_commands(
        &mut commands,
        "privilege",
        &apply.privilege.commands,
        &port.desired_privilege().apply_operations(),
        dry_run,
        runner,
    );
    extend_commands(
        &mut commands,
        "packages",
        &apply.packages.commands,
        &port.desired_packages().apply_operations(),
        dry_run,
        runner,
    );
    extend_commands(
        &mut commands,
        "services",
        &apply.services.commands,
        &port.desired_services().apply_operations(),
        dry_run,
        runner,
    );

    let executed_failure = commands
        .iter()
        .any(|item| item.executed && item.result.as_ref().is_some_and(|result| !result.success));

    HostApplyExecutionReport {
        schema: HOST_APPLY_EXECUTION_SCHEMA,
        host_resolution: apply.host_resolution.clone(),
        status: if dry_run {
            apply.status
        } else if executed_failure {
            SurfaceStatus::Blocked
        } else {
            SurfaceStatus::Active
        },
        dry_run,
        blocked_domains: apply.blocked_domains,
        next_actions: apply.next_actions,
        commands,
    }
}

fn overall_status(observed: &HostObservationReports) -> SurfaceStatus {
    observed.status
}

fn blocked_domains(observed: &HostObservationReports) -> Vec<&'static str> {
    observed.blocked_domains.clone()
}

fn blocked_domain_statuses(
    principal: &SurfaceStatus,
    privilege: &SurfaceStatus,
    packages: &SurfaceStatus,
    services: &SurfaceStatus,
) -> Vec<&'static str> {
    let mut blocked = Vec::new();
    if *principal != SurfaceStatus::Active {
        blocked.push("principal");
    }
    if *privilege != SurfaceStatus::Active {
        blocked.push("privilege");
    }
    if *packages != SurfaceStatus::Active {
        blocked.push("packages");
    }
    if *services != SurfaceStatus::Active {
        blocked.push("services");
    }
    blocked
}

fn next_actions(blocked_domains: &[&'static str]) -> Vec<&'static str> {
    let mut actions = Vec::new();
    if blocked_domains.contains(&"principal") {
        actions.push("repair_host_principal");
    }
    if blocked_domains.contains(&"privilege") {
        actions.push("repair_host_privilege_boundary");
    }
    if blocked_domains.contains(&"packages") {
        actions.push("repair_host_bootstrap_packages");
    }
    if blocked_domains.contains(&"services") {
        actions.push("repair_host_runtime_services");
    }
    actions
}

fn extend_commands(
    target: &mut Vec<ExecutedHostCommand>,
    domain: &'static str,
    commands: &[HostCommandSurface],
    operations: &[HostOperation],
    dry_run: bool,
    runner: &impl HostCommandRunnerPort,
) {
    for (command, operation) in commands.iter().zip(operations.iter()) {
        if dry_run {
            target.push(ExecutedHostCommand {
                domain,
                command: render_command(command),
                executed: false,
                result: None,
            });
            continue;
        }

        let result = runner.run_operation(operation);
        target.push(ExecutedHostCommand {
            domain,
            command: result.command.clone(),
            executed: true,
            result: Some(executed_result(result)),
        });
    }
}

fn executed_result(result: HostCommandResult) -> ExecutedHostCommandResult {
    ExecutedHostCommandResult {
        success: result.success,
        exit_code: result.exit_code,
        stdout: result.stdout,
        stderr: result.stderr,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HOST_ACTIVATION_PLAN_SCHEMA, HOST_APPLY_EXECUTION_SCHEMA, HOST_APPLY_REPORT_SCHEMA,
        HOST_BUNDLE_SCHEMA, HOST_OBSERVATION_SCHEMA, build_host_activation_plan,
        build_host_apply_report, build_host_bundle, collect_contracts, execute_host_apply,
        observe_host,
    };
    use crate::command_surface::{HostCommandInvocation, HostCommandSurface, render_command};
    use crate::domain::filesystem::FilesystemPolicy;
    use crate::domain::packages::PackageObservedState;
    use crate::domain::packages::PackagePolicy;
    use crate::domain::principal::PrincipalObservedState;
    use crate::domain::principal::PrincipalPolicy;
    use crate::domain::privilege::PrivilegeObservedState;
    use crate::domain::privilege::PrivilegePolicy;
    use crate::domain::services::{
        ServiceManagerKind, ServiceObservedState, ServiceObservedUnit, ServicePolicy,
        ServiceUnitPolicy,
    };
    use crate::domain::shared::{SubstrateBundleFile, SubstrateDomainContract, SurfaceStatus};
    use crate::ports::{
        HostCommandResult, HostCommandRunnerPort, HostContractPort, HostFilesystemPort,
        HostOperation, HostPackageObservationPort, HostPackagePort, HostPrincipalObservationPort,
        HostPrincipalPort, HostPrivilegeObservationPort, HostPrivilegePort,
        HostServiceObservationPort, HostServicePort,
    };
    use crate::runtime_layout::current_runtime_layout;

    struct StaticHostContractPort;
    struct FakeCommandRunner;
    struct FakeObservationAdapter;

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
    fn apply_report_propagates_dry_run_and_blockers() {
        let observed = observe_host(&StaticHostContractPort, &FakeObservationAdapter);
        let report = build_host_apply_report(&StaticHostContractPort, &observed, true);
        assert_eq!(report.schema, HOST_APPLY_REPORT_SCHEMA);
        assert!(report.dry_run);
        assert_eq!(report.status, SurfaceStatus::Blocked);
        assert!(!report.blocked_domains.is_empty());
    }

    #[test]
    fn execute_host_apply_in_dry_run_emits_commands_without_running_them() {
        let observed = observe_host(&StaticHostContractPort, &FakeObservationAdapter);
        let report =
            execute_host_apply(&StaticHostContractPort, &observed, true, &FakeCommandRunner);
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
            item.result.as_ref().is_some_and(|result| {
                !result.success && result.stderr == "package source unresolved"
            })
        }));
    }
}
