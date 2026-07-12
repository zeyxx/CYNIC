use serde::{Deserialize, Serialize};

use crate::command_surface::{
    HostCommandSurface, argv_command, install_dir_command, render_command, validate_sudoers_command,
};
use crate::domain::shared::{
    HostSubstrateDomain, SubstrateBundleFile, SubstrateDomainContract, SubstrateEvidence,
    SurfaceContract, SurfaceStatus,
};
use crate::install_contract::{MODE_0755, root_owned_path};
use crate::ports::HostOperation;
use crate::runtime_layout::current_runtime_layout;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivilegePolicy {
    pub sudoers_subject: String,
    pub helper_dir: String,
    pub config_dir: String,
    pub sudoers_dir: String,
    pub denial_proof_required: bool,
    pub dedicated_helpers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivilegeStatusReport {
    pub status: SurfaceStatus,
    pub helper_dir_present: bool,
    pub sudoers_projection_present: bool,
    pub denial_proof_ready: bool,
    pub evidences: Vec<SubstrateEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivilegeBundleReport {
    pub status: SurfaceStatus,
    pub files: Vec<SubstrateBundleFile>,
    pub commands: Vec<HostCommandSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivilegeActivationPlan {
    pub status: SurfaceStatus,
    pub preflight: Vec<HostCommandSurface>,
    pub install_commands: Vec<HostCommandSurface>,
    pub postflight: Vec<HostCommandSurface>,
    pub denial_checks: Vec<HostCommandSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivilegeApplyReport {
    pub status: SurfaceStatus,
    pub dry_run: bool,
    pub commands: Vec<HostCommandSurface>,
    pub proofs_required: Vec<String>,
    pub denial_proofs_required: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivilegeObservedState {
    pub helper_dir_present: bool,
    pub sudoers_projection_present: bool,
    pub denial_proof_ready: bool,
}

impl PrivilegePolicy {
    pub fn contract(&self) -> SubstrateDomainContract {
        let mut required_surfaces = vec![
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
        ];
        required_surfaces.push(SurfaceContract {
            surface: "denial_proof".to_string(),
            required: self.denial_proof_required,
        });

        SubstrateDomainContract {
            domain: HostSubstrateDomain::Privilege,
            status: SurfaceStatus::Planned,
            required_surfaces,
            evidences: vec![SubstrateEvidence {
                id: "privilege_policy_declared".to_string(),
                status: SurfaceStatus::Planned,
                detail: format!(
                    "sudoers_subject={} helper_dir={} helpers={}",
                    self.sudoers_subject,
                    self.helper_dir,
                    self.dedicated_helpers.join(",")
                ),
            }],
        }
    }

    pub fn bundle(&self) -> PrivilegeBundleReport {
        let mut files = self
            .dedicated_helpers
            .iter()
            .cloned()
            .map(|helper| SubstrateBundleFile {
                logical_name: "helper-binary".to_string(),
                install_path: format!("{}/{}", self.helper_dir, helper),
                mode: MODE_0755.octal.to_string(),
            })
            .collect::<Vec<_>>();
        files.push(SubstrateBundleFile {
            logical_name: "sudoers-dir".to_string(),
            install_path: self.sudoers_dir.clone(),
            mode: MODE_0755.octal.to_string(),
        });
        PrivilegeBundleReport {
            status: SurfaceStatus::Planned,
            files,
            commands: self.install_commands(),
        }
    }

    pub fn activation_plan(&self) -> PrivilegeActivationPlan {
        let runtime_layout = current_runtime_layout();
        let capabilities = runtime_layout.capabilities();
        let sudo_bin = runtime_layout.sudo_bin();
        PrivilegeActivationPlan {
            status: SurfaceStatus::Planned,
            preflight: vec![
                argv_command(
                    "query_subject",
                    vec!["id".to_string(), self.sudoers_subject.clone()],
                ),
                argv_command(
                    "check_helper_dir",
                    vec![
                        "test".to_string(),
                        "-d".to_string(),
                        self.helper_dir.clone(),
                    ],
                ),
            ],
            install_commands: self.install_commands(),
            postflight: vec![
                validate_sudoers_command(
                    "validate_sudoers",
                    format!("{}/<projection>", self.sudoers_dir),
                ),
                argv_command(
                    "prove_subject_sudo",
                    vec![
                        sudo_bin.clone(),
                        "-n".to_string(),
                        "-u".to_string(),
                        self.sudoers_subject.clone(),
                        "true".to_string(),
                    ],
                ),
            ],
            denial_checks: vec![
                argv_command(
                    "deny_route",
                    vec![
                        sudo_bin.clone(),
                        "-n".to_string(),
                        capabilities.route_tool.command.clone(),
                        "route".to_string(),
                        "show".to_string(),
                    ],
                ),
                argv_command(
                    "deny_service_status",
                    vec![
                        sudo_bin.clone(),
                        "-n".to_string(),
                        capabilities.service_manager.command.clone(),
                        "status".to_string(),
                        "cynic-node.service".to_string(),
                    ],
                ),
                argv_command(
                    "deny_shell",
                    vec![
                        sudo_bin,
                        "-n".to_string(),
                        "/bin/sh".to_string(),
                        "-c".to_string(),
                        "true".to_string(),
                    ],
                ),
            ],
        }
    }

    pub fn apply_report(&self, dry_run: bool) -> PrivilegeApplyReport {
        let activation_plan = self.activation_plan();
        PrivilegeApplyReport {
            status: SurfaceStatus::Planned,
            dry_run,
            commands: self.install_commands(),
            proofs_required: vec![
                format!("test -d {}", self.helper_dir),
                format!("visudo -cf {}/<projection>", self.sudoers_dir),
            ],
            denial_proofs_required: activation_plan
                .denial_checks
                .iter()
                .map(render_command)
                .collect(),
        }
    }

    pub fn status(&self, observed: &PrivilegeObservedState) -> PrivilegeStatusReport {
        let status = if observed.helper_dir_present
            && observed.sudoers_projection_present
            && (!self.denial_proof_required || observed.denial_proof_ready)
        {
            SurfaceStatus::Active
        } else {
            SurfaceStatus::Blocked
        };
        PrivilegeStatusReport {
            status,
            helper_dir_present: observed.helper_dir_present,
            sudoers_projection_present: observed.sudoers_projection_present,
            denial_proof_ready: observed.denial_proof_ready,
            evidences: vec![
                SubstrateEvidence {
                    id: "privilege_helper_dir".to_string(),
                    status: if observed.helper_dir_present {
                        SurfaceStatus::Active
                    } else {
                        SurfaceStatus::Blocked
                    },
                    detail: self.helper_dir.clone(),
                },
                SubstrateEvidence {
                    id: "privilege_denial_proof".to_string(),
                    status: if !self.denial_proof_required || observed.denial_proof_ready {
                        SurfaceStatus::Active
                    } else {
                        SurfaceStatus::Blocked
                    },
                    detail: format!("required={}", self.denial_proof_required),
                },
            ],
        }
    }

    fn install_commands(&self) -> Vec<HostCommandSurface> {
        vec![
            install_dir_command(
                "prepare_directory",
                self.helper_dir.clone(),
                root_owned_path(MODE_0755),
            ),
            install_dir_command(
                "prepare_directory",
                self.config_dir.clone(),
                root_owned_path(MODE_0755),
            ),
            install_dir_command(
                "prepare_directory",
                self.sudoers_dir.clone(),
                root_owned_path(MODE_0755),
            ),
        ]
    }

    pub fn apply_operations(&self) -> Vec<HostOperation> {
        vec![
            HostOperation::EnsureDir {
                path: self.helper_dir.clone(),
                mode: MODE_0755.unix,
            },
            HostOperation::EnsureDir {
                path: self.config_dir.clone(),
                mode: MODE_0755.unix,
            },
            HostOperation::EnsureDir {
                path: self.sudoers_dir.clone(),
                mode: MODE_0755.unix,
            },
        ]
    }
}
