use serde::{Deserialize, Serialize};

use crate::command_surface::{HostCommandSurface, argv_command, install_dir_command};
use crate::domain::shared::{
    HostSubstrateDomain, SubstrateBundleFile, SubstrateDomainContract, SubstrateEvidence,
    SurfaceContract, SurfaceStatus,
};
use crate::install_contract::{MODE_0755, root_owned_path};
use crate::ports::HostOperation;
use crate::runtime_layout::current_runtime_layout;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalPolicy {
    pub runtime_user: String,
    pub runtime_group: String,
    pub home_dir: String,
    pub config_dir: String,
    pub state_dirs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalStatusReport {
    pub status: SurfaceStatus,
    pub user_present: bool,
    pub group_present: bool,
    pub home_matches: bool,
    pub missing_state_dirs: Vec<String>,
    pub evidences: Vec<SubstrateEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalBundleReport {
    pub status: SurfaceStatus,
    pub files: Vec<SubstrateBundleFile>,
    pub commands: Vec<HostCommandSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalActivationPlan {
    pub status: SurfaceStatus,
    pub preflight: Vec<HostCommandSurface>,
    pub install_commands: Vec<HostCommandSurface>,
    pub postflight: Vec<HostCommandSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalApplyReport {
    pub status: SurfaceStatus,
    pub dry_run: bool,
    pub commands: Vec<HostCommandSurface>,
    pub proofs_required: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalObservedState {
    pub user_present: bool,
    pub group_present: bool,
    pub observed_home_dir: Option<String>,
    pub present_state_dirs: Vec<String>,
}

impl PrincipalPolicy {
    pub fn contract(&self) -> SubstrateDomainContract {
        SubstrateDomainContract {
            domain: HostSubstrateDomain::Principal,
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
                id: "principal_policy_declared".to_string(),
                status: SurfaceStatus::Planned,
                detail: format!(
                    "runtime_user={} runtime_group={} home_dir={}",
                    self.runtime_user, self.runtime_group, self.home_dir
                ),
            }],
        }
    }

    pub fn bundle(&self) -> PrincipalBundleReport {
        let mut files = vec![SubstrateBundleFile {
            logical_name: "home-dir".to_string(),
            install_path: self.home_dir.clone(),
            mode: MODE_0755.octal.to_string(),
        }];
        files.extend(
            self.state_dirs
                .iter()
                .cloned()
                .map(|path| SubstrateBundleFile {
                    logical_name: "state-dir".to_string(),
                    install_path: path,
                    mode: MODE_0755.octal.to_string(),
                }),
        );

        PrincipalBundleReport {
            status: SurfaceStatus::Planned,
            files,
            commands: self.install_commands(),
        }
    }

    pub fn activation_plan(&self) -> PrincipalActivationPlan {
        let runtime_layout = current_runtime_layout();
        PrincipalActivationPlan {
            status: SurfaceStatus::Planned,
            preflight: vec![
                argv_command(
                    "query_user",
                    vec![
                        runtime_layout.getent_bin(),
                        "passwd".to_string(),
                        self.runtime_user.clone(),
                    ],
                ),
                argv_command(
                    "query_group",
                    vec![
                        runtime_layout.getent_bin(),
                        "group".to_string(),
                        self.runtime_group.clone(),
                    ],
                ),
            ],
            install_commands: self.install_commands(),
            postflight: vec![
                argv_command(
                    "query_user",
                    vec![
                        runtime_layout.getent_bin(),
                        "passwd".to_string(),
                        self.runtime_user.clone(),
                    ],
                ),
                argv_command(
                    "check_home_dir",
                    vec!["test".to_string(), "-d".to_string(), self.home_dir.clone()],
                ),
                argv_command(
                    "inspect_home_dir",
                    vec![
                        "namei".to_string(),
                        "-om".to_string(),
                        self.home_dir.clone(),
                    ],
                ),
            ],
        }
    }

    pub fn apply_report(&self, dry_run: bool) -> PrincipalApplyReport {
        PrincipalApplyReport {
            status: SurfaceStatus::Planned,
            dry_run,
            commands: self.install_commands(),
            proofs_required: vec![
                format!("getent passwd {}", self.runtime_user),
                format!("getent group {}", self.runtime_group),
                format!("test -d {}", self.home_dir),
            ],
        }
    }

    pub fn status(&self, observed: &PrincipalObservedState) -> PrincipalStatusReport {
        let home_matches = observed.observed_home_dir.as_deref() == Some(self.home_dir.as_str());
        let missing_state_dirs = self
            .state_dirs
            .iter()
            .filter(|path| !observed.present_state_dirs.contains(*path))
            .cloned()
            .collect::<Vec<_>>();
        let status = if observed.user_present
            && observed.group_present
            && home_matches
            && missing_state_dirs.is_empty()
        {
            SurfaceStatus::Active
        } else {
            SurfaceStatus::Blocked
        };
        PrincipalStatusReport {
            status,
            user_present: observed.user_present,
            group_present: observed.group_present,
            home_matches,
            missing_state_dirs,
            evidences: vec![
                SubstrateEvidence {
                    id: "principal_user_presence".to_string(),
                    status: if observed.user_present {
                        SurfaceStatus::Active
                    } else {
                        SurfaceStatus::Blocked
                    },
                    detail: format!("runtime_user={}", self.runtime_user),
                },
                SubstrateEvidence {
                    id: "principal_home_alignment".to_string(),
                    status: if home_matches {
                        SurfaceStatus::Active
                    } else {
                        SurfaceStatus::Blocked
                    },
                    detail: format!("expected_home={}", self.home_dir),
                },
            ],
        }
    }

    pub fn apply_operations(&self) -> Vec<HostOperation> {
        let mut operations = vec![HostOperation::EnsureDir {
            path: self.config_dir.clone(),
            mode: MODE_0755.unix,
        }];
        operations.extend(
            self.state_dirs
                .iter()
                .cloned()
                .map(|path| HostOperation::EnsureDir {
                    path,
                    mode: MODE_0755.unix,
                }),
        );
        operations
    }

    fn install_commands(&self) -> Vec<HostCommandSurface> {
        let mut commands = vec![install_dir_command(
            "prepare_directory",
            self.config_dir.clone(),
            root_owned_path(MODE_0755),
        )];
        commands.extend(self.state_dirs.iter().map(|path| {
            install_dir_command(
                "prepare_directory",
                path.clone(),
                root_owned_path(MODE_0755),
            )
        }));
        commands
    }
}
