use serde::{Deserialize, Serialize};

use crate::command_surface::{HostCommandSurface, argv_command, install_dir_command};
use crate::domain::shared::{
    BoundaryKind, HostSubstrateDomain, LifecycleDisposition, LifecycleProjection, LifecycleStatus,
    SubstrateBundleFile, SubstrateDomainContract, SubstrateEvidence, SurfaceContract,
    SurfaceStatus,
};
use crate::install_contract::{MODE_0644, MODE_0755, root_owned_path};
use crate::ports::HostOperation;
use crate::runtime_layout::current_runtime_layout;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceManagerKind {
    Systemd,
    OpenRc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceUnitPolicy {
    #[serde(default)]
    pub service_id: String,
    pub unit_name: String,
    pub enable_on_boot: bool,
    pub restart_mode: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub health_port: Option<u16>,
    #[serde(default)]
    pub process_name: Option<String>,
    #[serde(default)]
    pub supervisor_provider: Option<String>,
    #[serde(default)]
    pub health_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServicePolicy {
    pub manager: ServiceManagerKind,
    pub unit_dir: String,
    pub units: Vec<ServiceUnitPolicy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceStatusReport {
    pub status: SurfaceStatus,
    pub lifecycle_status: LifecycleStatus,
    pub lifecycle_disposition: LifecycleDisposition,
    pub next_boundary_kind: BoundaryKind,
    pub next_boundary_reason: String,
    pub next_command: Option<String>,
    pub missing_units: Vec<String>,
    pub disabled_units: Vec<String>,
    pub inactive_units: Vec<String>,
    pub evidences: Vec<SubstrateEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceBundleReport {
    pub status: SurfaceStatus,
    pub files: Vec<SubstrateBundleFile>,
    pub commands: Vec<HostCommandSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceActivationPlan {
    pub status: SurfaceStatus,
    pub preflight: Vec<HostCommandSurface>,
    pub install_commands: Vec<HostCommandSurface>,
    pub postflight: Vec<HostCommandSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceApplyReport {
    pub status: SurfaceStatus,
    pub dry_run: bool,
    pub commands: Vec<HostCommandSurface>,
    pub proofs_required: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceObservedUnit {
    pub unit_name: String,
    pub present: bool,
    pub enabled: bool,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceObservedState {
    pub units: Vec<ServiceObservedUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCommandContext {
    pub systemctl_bin: String,
    pub rc_service_bin: String,
    pub rc_update_bin: String,
}

impl ServiceCommandContext {
    pub fn from_current_runtime() -> Self {
        let layout = current_runtime_layout();
        Self {
            systemctl_bin: layout.systemctl_bin(),
            rc_service_bin: layout.rc_service_bin(),
            rc_update_bin: layout.rc_update_bin(),
        }
    }
}

impl ServicePolicy {
    pub fn contract(&self) -> SubstrateDomainContract {
        SubstrateDomainContract {
            domain: HostSubstrateDomain::Services,
            status: SurfaceStatus::Planned,
            required_surfaces: vec![
                SurfaceContract {
                    surface: "status".to_string(),
                    required: true,
                },
                SurfaceContract {
                    surface: "bundle".to_string(),
                    required: true,
                },
                SurfaceContract {
                    surface: "activation_plan".to_string(),
                    required: true,
                },
                SurfaceContract {
                    surface: "apply".to_string(),
                    required: true,
                },
                SurfaceContract {
                    surface: "proof".to_string(),
                    required: true,
                },
            ],
            evidences: vec![SubstrateEvidence {
                id: "service_policy_declared".to_string(),
                status: SurfaceStatus::Planned,
                detail: format!("units={}", self.units.len()),
            }],
        }
    }

    pub fn bundle(&self) -> ServiceBundleReport {
        self.bundle_with_context(&ServiceCommandContext::from_current_runtime())
    }

    pub fn bundle_with_context(&self, context: &ServiceCommandContext) -> ServiceBundleReport {
        ServiceBundleReport {
            status: SurfaceStatus::Planned,
            files: self
                .units
                .iter()
                .map(|unit| SubstrateBundleFile {
                    logical_name: self.unit_logical_name().to_string(),
                    install_path: format!("{}/{}", self.unit_dir, self.install_unit_name(unit)),
                    mode: MODE_0644.octal.to_string(),
                })
                .collect(),
            commands: self.install_commands(context),
        }
    }

    pub fn activation_plan(&self) -> ServiceActivationPlan {
        self.activation_plan_with_context(&ServiceCommandContext::from_current_runtime())
    }

    pub fn activation_plan_with_context(
        &self,
        context: &ServiceCommandContext,
    ) -> ServiceActivationPlan {
        ServiceActivationPlan {
            status: SurfaceStatus::Planned,
            preflight: self
                .units
                .iter()
                .map(|unit| self.status_command(&unit.unit_name, context))
                .collect(),
            install_commands: self.install_commands(context),
            postflight: self
                .units
                .iter()
                .flat_map(|unit| {
                    [
                        self.is_enabled_command(&unit.unit_name, context),
                        self.is_active_command(&unit.unit_name, context),
                    ]
                })
                .collect(),
        }
    }

    pub fn apply_report(&self, dry_run: bool) -> ServiceApplyReport {
        self.apply_report_with_context(dry_run, &ServiceCommandContext::from_current_runtime())
    }

    pub fn apply_report_with_context(
        &self,
        dry_run: bool,
        context: &ServiceCommandContext,
    ) -> ServiceApplyReport {
        ServiceApplyReport {
            status: SurfaceStatus::Planned,
            dry_run,
            commands: self.install_commands(context),
            proofs_required: self
                .units
                .iter()
                .flat_map(|unit| {
                    [
                        format!("unit_enabled:{}", unit.unit_name),
                        format!("unit_active:{}", unit.unit_name),
                    ]
                })
                .collect(),
        }
    }

    pub fn status(&self, observed: &ServiceObservedState) -> ServiceStatusReport {
        let mut missing_units = Vec::new();
        let mut disabled_units = Vec::new();
        let mut inactive_units = Vec::new();

        for expected in &self.units {
            match observed
                .units
                .iter()
                .find(|unit| unit.unit_name == expected.unit_name)
            {
                Some(unit) => {
                    if !unit.present {
                        missing_units.push(expected.unit_name.clone());
                    }
                    if expected.enable_on_boot && !unit.enabled {
                        disabled_units.push(expected.unit_name.clone());
                    }
                    if !unit.active {
                        inactive_units.push(expected.unit_name.clone());
                    }
                }
                None => missing_units.push(expected.unit_name.clone()),
            }
        }

        let status =
            if missing_units.is_empty() && disabled_units.is_empty() && inactive_units.is_empty() {
                SurfaceStatus::Active
            } else {
                SurfaceStatus::Blocked
            };
        let lifecycle_status = if !missing_units.is_empty() {
            LifecycleStatus::Missing
        } else if !disabled_units.is_empty() || !inactive_units.is_empty() {
            LifecycleStatus::PrincipalMismatch
        } else {
            LifecycleStatus::Ready
        };
        let projection = match lifecycle_status {
            LifecycleStatus::Ready => LifecycleProjection::ready_observe(
                "declared services are present, enabled as required, and active",
            ),
            LifecycleStatus::Missing => LifecycleProjection::missing_adopt(
                "one or more declared service units are missing from observation",
                Some("cynic host network-baseline plan".to_string()),
            ),
            LifecycleStatus::PrincipalMismatch => LifecycleProjection::principal_mismatch_adopt(
                "declared service units are present but not yet enabled or active as required",
                Some("cynic host network-baseline plan".to_string()),
            ),
            LifecycleStatus::ConnectionRequired => {
                LifecycleProjection::connection_required_observe(
                    "service lifecycle does not model a connection boundary",
                )
            }
        };

        ServiceStatusReport {
            status,
            lifecycle_status,
            lifecycle_disposition: projection.disposition,
            next_boundary_kind: projection.next_boundary_kind,
            next_boundary_reason: projection.next_boundary_reason,
            next_command: projection.next_command,
            missing_units,
            disabled_units,
            inactive_units,
<<<<<<< HEAD
            evidences: {
                let mut names = Vec::with_capacity(self.units.len());
                for unit in &self.units {
                    names.push(unit.unit_name.as_str());
                }
                vec![SubstrateEvidence {
                    id: "service_units_declared".to_string(),
                    status: SurfaceStatus::Planned,
                    detail: names.join(","),
                }]
            },
=======
            evidences: vec![SubstrateEvidence {
                id: "service_units_declared".to_string(),
                status: SurfaceStatus::Planned,
                detail: self
                    .units
                    .iter()
                    .map(|unit| unit.unit_name.clone())
                    .collect::<Vec<_>>()
                    .join(","),
            }],
>>>>>>> github/main
        }
    }

    fn install_commands(&self, context: &ServiceCommandContext) -> Vec<HostCommandSurface> {
        let mut commands = vec![install_dir_command(
            "prepare_directory",
            self.unit_dir.clone(),
            root_owned_path(MODE_0755),
        )];
        if self.manager == ServiceManagerKind::Systemd {
            commands.push(argv_command(
                "daemon_reload",
                vec![context.systemctl_bin.clone(), "daemon-reload".to_string()],
            ));
        }
        commands.extend(
            self.units
                .iter()
                .flat_map(|unit| self.unit_install_commands(unit, context)),
        );
        commands
    }

    pub fn apply_operations(&self) -> Vec<HostOperation> {
        let mut operations = vec![HostOperation::EnsureDir {
            path: self.unit_dir.clone(),
            mode: MODE_0755.unix,
        }];
        if self.manager == ServiceManagerKind::Systemd {
            operations.push(HostOperation::ServiceManagerDaemonReload);
        }
        operations.extend(self.units.iter().map(|unit| {
            if unit.enable_on_boot {
                HostOperation::ServiceEnableNow {
                    unit: unit.unit_name.clone(),
                }
            } else {
                HostOperation::ServiceStart {
                    unit: unit.unit_name.clone(),
                }
            }
        }));
        operations
    }

    fn unit_logical_name(&self) -> &'static str {
        match self.manager {
            ServiceManagerKind::Systemd => "systemd-unit",
            ServiceManagerKind::OpenRc => "openrc-service",
        }
    }

    fn install_unit_name(&self, unit: &ServiceUnitPolicy) -> String {
        match self.manager {
            ServiceManagerKind::Systemd => unit.unit_name.clone(),
            ServiceManagerKind::OpenRc => openrc_service_name(&unit.unit_name),
        }
    }

    fn status_command(&self, unit: &str, context: &ServiceCommandContext) -> HostCommandSurface {
        match self.manager {
            ServiceManagerKind::Systemd => argv_command(
                "service_status",
                vec![
                    context.systemctl_bin.clone(),
                    "status".to_string(),
                    unit.to_string(),
                ],
            ),
            ServiceManagerKind::OpenRc => argv_command(
                "service_status",
                vec![
                    context.rc_service_bin.clone(),
                    openrc_service_name(unit),
                    "status".to_string(),
                ],
            ),
        }
    }

    fn is_enabled_command(
        &self,
        unit: &str,
        context: &ServiceCommandContext,
    ) -> HostCommandSurface {
        match self.manager {
            ServiceManagerKind::Systemd => argv_command(
                "service_is_enabled",
                vec![
                    context.systemctl_bin.clone(),
                    "is-enabled".to_string(),
                    unit.to_string(),
                ],
            ),
            ServiceManagerKind::OpenRc => argv_command(
                "service_is_enabled",
                vec![
                    context.rc_update_bin.clone(),
                    "show".to_string(),
                    openrc_service_name(unit),
                ],
            ),
        }
    }

    fn is_active_command(&self, unit: &str, context: &ServiceCommandContext) -> HostCommandSurface {
        match self.manager {
            ServiceManagerKind::Systemd => argv_command(
                "service_is_active",
                vec![
                    context.systemctl_bin.clone(),
                    "is-active".to_string(),
                    unit.to_string(),
                ],
            ),
            ServiceManagerKind::OpenRc => argv_command(
                "service_is_active",
                vec![
                    context.rc_service_bin.clone(),
                    openrc_service_name(unit),
                    "status".to_string(),
                ],
            ),
        }
    }

    fn unit_install_commands(
        &self,
        unit: &ServiceUnitPolicy,
        context: &ServiceCommandContext,
    ) -> Vec<HostCommandSurface> {
        if unit.enable_on_boot {
            self.enable_now_commands(&unit.unit_name, context)
        } else {
            vec![self.start_command(&unit.unit_name, context)]
        }
    }

    fn enable_now_commands(
        &self,
        unit: &str,
        context: &ServiceCommandContext,
    ) -> Vec<HostCommandSurface> {
        match self.manager {
            ServiceManagerKind::Systemd => vec![argv_command(
                "enable_now",
                vec![
                    context.systemctl_bin.clone(),
                    "enable".to_string(),
                    "--now".to_string(),
                    unit.to_string(),
                ],
            )],
            ServiceManagerKind::OpenRc => {
                let service = openrc_service_name(unit);
                vec![
                    argv_command(
                        "enable_on_boot",
                        vec![
                            context.rc_update_bin.clone(),
                            "add".to_string(),
                            service.clone(),
                            "default".to_string(),
                        ],
                    ),
                    argv_command(
                        "start_service",
                        vec![context.rc_service_bin.clone(), service, "start".to_string()],
                    ),
                ]
            }
        }
    }

    fn start_command(&self, unit: &str, context: &ServiceCommandContext) -> HostCommandSurface {
        match self.manager {
            ServiceManagerKind::Systemd => argv_command(
                "start_service",
                vec![
                    context.systemctl_bin.clone(),
                    "start".to_string(),
                    unit.to_string(),
                ],
            ),
            ServiceManagerKind::OpenRc => argv_command(
                "start_service",
                vec![
                    context.rc_service_bin.clone(),
                    openrc_service_name(unit),
                    "start".to_string(),
                ],
            ),
        }
    }
}

pub(crate) fn openrc_service_name(unit: &str) -> String {
    unit.strip_suffix(".service").unwrap_or(unit).to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        ServiceCommandContext, ServiceManagerKind, ServicePolicy, ServiceUnitPolicy,
        openrc_service_name,
    };
    use crate::command_surface::render_command;

    fn context() -> ServiceCommandContext {
        ServiceCommandContext {
            systemctl_bin: "/portable/bin/systemctl".to_string(),
            rc_service_bin: "/portable/bin/rc-service".to_string(),
            rc_update_bin: "/portable/bin/rc-update".to_string(),
        }
    }

    fn unit() -> ServiceUnitPolicy {
        ServiceUnitPolicy {
            service_id: "node".to_string(),
            unit_name: "cynic-node.service".to_string(),
            enable_on_boot: true,
            restart_mode: "on-failure".to_string(),
            source: "test".to_string(),
            health_port: None,
            process_name: Some("cynic-node".to_string()),
            supervisor_provider: Some("openrc".to_string()),
            health_kind: None,
        }
    }

    #[test]
    fn openrc_service_name_strips_systemd_suffix() {
        assert_eq!(openrc_service_name("cynic-node.service"), "cynic-node");
        assert_eq!(openrc_service_name("cynic-node"), "cynic-node");
    }

    #[test]
    fn openrc_activation_plan_uses_rc_tools_not_systemctl() {
        let policy = ServicePolicy {
            manager: ServiceManagerKind::OpenRc,
            unit_dir: "/etc/init.d".to_string(),
            units: vec![unit()],
        };

        let plan = policy.activation_plan_with_context(&context());
        let rendered = plan
            .install_commands
            .iter()
            .chain(plan.postflight.iter())
            .map(render_command)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("rc-update add cynic-node default"));
        assert!(rendered.contains("rc-service cynic-node start"));
        assert!(rendered.contains("rc-update show cynic-node"));
        assert!(rendered.contains("rc-service cynic-node status"));
        assert!(!rendered.contains("systemctl"));
    }

    #[test]
    fn openrc_bundle_targets_init_d_services() {
        let policy = ServicePolicy {
            manager: ServiceManagerKind::OpenRc,
            unit_dir: "/etc/init.d".to_string(),
            units: vec![unit()],
        };

        let bundle = policy.bundle_with_context(&context());
        let rendered = bundle
            .commands
            .iter()
            .map(render_command)
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(bundle.files[0].logical_name, "openrc-service");
        assert_eq!(bundle.files[0].install_path, "/etc/init.d/cynic-node");
        assert!(rendered.contains("rc-update add cynic-node default"));
        assert!(rendered.contains("rc-service cynic-node start"));
    }

    #[test]
    fn systemd_activation_plan_uses_injected_command_context() {
        let policy = ServicePolicy {
            manager: ServiceManagerKind::Systemd,
            unit_dir: "/etc/systemd/system".to_string(),
            units: vec![unit()],
        };

        let plan = policy.activation_plan_with_context(&context());
        let rendered = plan
            .preflight
            .iter()
            .chain(plan.install_commands.iter())
            .chain(plan.postflight.iter())
            .map(render_command)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("/portable/bin/systemctl status cynic-node.service"));
        assert!(rendered.contains("/portable/bin/systemctl daemon-reload"));
        assert!(rendered.contains("/portable/bin/systemctl enable --now cynic-node.service"));
        assert!(rendered.contains("/portable/bin/systemctl is-enabled cynic-node.service"));
        assert!(rendered.contains("/portable/bin/systemctl is-active cynic-node.service"));
    }

    #[test]
    fn apply_report_uses_injected_service_command_context() {
        let policy = ServicePolicy {
            manager: ServiceManagerKind::OpenRc,
            unit_dir: "/etc/init.d".to_string(),
            units: vec![unit()],
        };

        let report = policy.apply_report_with_context(true, &context());
        let rendered = report
            .commands
            .iter()
            .map(render_command)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("/portable/bin/rc-update add cynic-node default"));
        assert!(rendered.contains("/portable/bin/rc-service cynic-node start"));
    }
}
