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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalCommandContext {
    pub getent_bin: String,
}

impl PrincipalCommandContext {
    pub fn from_current_runtime() -> Self {
        let runtime_layout = current_runtime_layout();
        Self {
            getent_bin: runtime_layout.getent_bin(),
        }
    }
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
        self.activation_plan_with_context(&PrincipalCommandContext::from_current_runtime())
    }

    pub fn activation_plan_with_context(
        &self,
        context: &PrincipalCommandContext,
    ) -> PrincipalActivationPlan {
        PrincipalActivationPlan {
            status: SurfaceStatus::Planned,
            preflight: vec![
                argv_command(
                    "query_user",
                    vec![
                        context.getent_bin.clone(),
                        "passwd".to_string(),
                        self.runtime_user.clone(),
                    ],
                ),
                argv_command(
                    "query_group",
                    vec![
                        context.getent_bin.clone(),
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
                        context.getent_bin.clone(),
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
        self.apply_report_with_context(dry_run, &PrincipalCommandContext::from_current_runtime())
    }

    pub fn apply_report_with_context(
        &self,
        dry_run: bool,
        context: &PrincipalCommandContext,
    ) -> PrincipalApplyReport {
        PrincipalApplyReport {
            status: SurfaceStatus::Planned,
            dry_run,
            commands: self.install_commands(),
            proofs_required: vec![
                format!("{} passwd {}", context.getent_bin, self.runtime_user),
                format!("{} group {}", context.getent_bin, self.runtime_group),
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

#[cfg(test)]
mod tests {
    use super::{PrincipalCommandContext, PrincipalObservedState, PrincipalPolicy};
    use crate::command_surface::render_command;
    use crate::domain::shared::SurfaceStatus;

    fn context() -> PrincipalCommandContext {
        PrincipalCommandContext {
            getent_bin: "/portable/bin/getent".to_string(),
        }
    }

    fn policy() -> PrincipalPolicy {
        PrincipalPolicy {
            runtime_user: "cynic".to_string(),
            runtime_group: "cynic".to_string(),
            home_dir: "/var/lib/cynic".to_string(),
            config_dir: "/etc/cynic".to_string(),
            state_dirs: vec!["/var/lib/cynic/state".to_string()],
        }
    }

    #[test]
    fn activation_plan_uses_injected_principal_command_context() {
        let activation = policy().activation_plan_with_context(&context());
        let rendered = activation
            .preflight
            .iter()
            .chain(activation.postflight.iter())
            .map(render_command)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("/portable/bin/getent passwd cynic"));
        assert!(rendered.contains("/portable/bin/getent group cynic"));
    }

    #[test]
    fn apply_report_uses_injected_principal_command_context() {
        let report = policy().apply_report_with_context(true, &context());
        let rendered = report.proofs_required.join("\n");

        assert!(rendered.contains("/portable/bin/getent passwd cynic"));
        assert!(rendered.contains("/portable/bin/getent group cynic"));
        assert!(rendered.contains("test -d /var/lib/cynic"));
    }

    #[test]
    fn status_blocks_when_home_does_not_match_policy() {
        let report = policy().status(&PrincipalObservedState {
            user_present: true,
            group_present: true,
            observed_home_dir: Some("/wrong".to_string()),
            present_state_dirs: vec!["/var/lib/cynic/state".to_string()],
        });

        assert_eq!(report.status, SurfaceStatus::Blocked);
    }
}
