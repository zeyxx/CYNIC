use serde::{Deserialize, Serialize};

use crate::command_surface::{HostCommandSurface, argv_command, install_dir_command};
use crate::domain::shared::{
    HostSubstrateDomain, SubstrateBundleFile, SubstrateDomainContract, SubstrateEvidence,
    SurfaceContract, SurfaceStatus,
};
use crate::install_contract::{MODE_0644, MODE_0755, root_owned_path};
use crate::ports::HostOperation;
use crate::runtime_layout::current_runtime_layout;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceManagerKind {
    Systemd,
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
        ServiceBundleReport {
            status: SurfaceStatus::Planned,
            files: self
                .units
                .iter()
                .map(|unit| SubstrateBundleFile {
                    logical_name: self.unit_logical_name().to_string(),
                    install_path: format!("{}/{}", self.unit_dir, unit.unit_name),
                    mode: MODE_0644.octal.to_string(),
                })
                .collect(),
            commands: self.install_commands(),
        }
    }

    pub fn activation_plan(&self) -> ServiceActivationPlan {
        ServiceActivationPlan {
            status: SurfaceStatus::Planned,
            preflight: self
                .units
                .iter()
                .map(|unit| self.status_command(&unit.unit_name))
                .collect(),
            install_commands: self.install_commands(),
            postflight: self
                .units
                .iter()
                .flat_map(|unit| {
                    [
                        self.is_enabled_command(&unit.unit_name),
                        self.is_active_command(&unit.unit_name),
                    ]
                })
                .collect(),
        }
    }

    pub fn apply_report(&self, dry_run: bool) -> ServiceApplyReport {
        ServiceApplyReport {
            status: SurfaceStatus::Planned,
            dry_run,
            commands: self.install_commands(),
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

        ServiceStatusReport {
            status,
            missing_units,
            disabled_units,
            inactive_units,
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
        }
    }

    fn install_commands(&self) -> Vec<HostCommandSurface> {
        let layout = current_runtime_layout();
        let mut commands = vec![install_dir_command(
            "prepare_directory",
            self.unit_dir.clone(),
            root_owned_path(MODE_0755),
        )];
        if self.manager == ServiceManagerKind::Systemd {
            commands.push(argv_command(
                "daemon_reload",
                vec![layout.systemctl_bin(), "daemon-reload".to_string()],
            ));
        }
        commands.extend(self.units.iter().map(|unit| {
            if unit.enable_on_boot {
                argv_command(
                    "enable_now",
                    vec![
                        layout.systemctl_bin(),
                        "enable".to_string(),
                        "--now".to_string(),
                        unit.unit_name.clone(),
                    ],
                )
            } else {
                argv_command(
                    "start_service",
                    vec![
                        layout.systemctl_bin(),
                        "start".to_string(),
                        unit.unit_name.clone(),
                    ],
                )
            }
        }));
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
        }
    }

    fn status_command(&self, unit: &str) -> HostCommandSurface {
        let layout = current_runtime_layout();
        match self.manager {
            ServiceManagerKind::Systemd => argv_command(
                "service_status",
                vec![
                    layout.systemctl_bin(),
                    "status".to_string(),
                    unit.to_string(),
                ],
            ),
        }
    }

    fn is_enabled_command(&self, unit: &str) -> HostCommandSurface {
        let layout = current_runtime_layout();
        match self.manager {
            ServiceManagerKind::Systemd => argv_command(
                "service_is_enabled",
                vec![
                    layout.systemctl_bin(),
                    "is-enabled".to_string(),
                    unit.to_string(),
                ],
            ),
        }
    }

    fn is_active_command(&self, unit: &str) -> HostCommandSurface {
        let layout = current_runtime_layout();
        match self.manager {
            ServiceManagerKind::Systemd => argv_command(
                "service_is_active",
                vec![
                    layout.systemctl_bin(),
                    "is-active".to_string(),
                    unit.to_string(),
                ],
            ),
        }
    }
}
