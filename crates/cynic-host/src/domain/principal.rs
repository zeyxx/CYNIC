use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::command_surface::{HostCommandSurface, argv_command, install_dir_command};
use crate::domain::shared::{
    BoundaryKind, HostSubstrateDomain, LifecycleDisposition, LifecycleProjection, LifecycleStatus,
    SubstrateBundleFile, SubstrateDomainContract, SubstrateEvidence, SurfaceContract,
    SurfaceStatus,
};
use crate::install_contract::{InstallOwnership, InstallPathPolicy, MODE_0755};
use crate::ports::{HostOperation, PrincipalLoginShell};
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
    pub lifecycle_status: LifecycleStatus,
    pub lifecycle_disposition: LifecycleDisposition,
    pub next_boundary_kind: BoundaryKind,
    pub next_boundary_reason: String,
    pub next_command: Option<String>,
    pub user_present: bool,
    pub group_present: bool,
    pub home_matches: bool,
    pub missing_state_dirs: Vec<String>,
    pub misowned_paths: Vec<String>,
<<<<<<< HEAD
    pub ownership_unknown_paths: Vec<String>,
=======
>>>>>>> github/main
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
    #[serde(default)]
    pub owned_paths: Vec<String>,
<<<<<<< HEAD
    #[serde(default)]
    pub ownership_unknown_paths: Vec<String>,
=======
>>>>>>> github/main
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
        let misowned_paths = self
            .managed_paths()
            .into_iter()
<<<<<<< HEAD
            .filter(|path| {
                !observed.owned_paths.contains(path)
                    && !observed.ownership_unknown_paths.contains(path)
            })
            .collect::<Vec<_>>();
        let ownership_unknown_paths = observed.ownership_unknown_paths.clone();
=======
            .filter(|path| !observed.owned_paths.contains(path))
            .collect::<Vec<_>>();
>>>>>>> github/main
        let status = if observed.user_present
            && observed.group_present
            && home_matches
            && missing_state_dirs.is_empty()
            && misowned_paths.is_empty()
<<<<<<< HEAD
            && ownership_unknown_paths.is_empty()
=======
>>>>>>> github/main
        {
            SurfaceStatus::Active
        } else {
            SurfaceStatus::Blocked
        };
        let lifecycle_status = if !observed.user_present
            || !observed.group_present
            || !missing_state_dirs.is_empty()
        {
            LifecycleStatus::Missing
        } else if !home_matches || !misowned_paths.is_empty() {
            LifecycleStatus::PrincipalMismatch
<<<<<<< HEAD
        } else if !ownership_unknown_paths.is_empty() {
            LifecycleStatus::ConnectionRequired
=======
>>>>>>> github/main
        } else {
            LifecycleStatus::Ready
        };
        let projection = match lifecycle_status {
            LifecycleStatus::Ready => LifecycleProjection::ready_observe(
                "runtime principal is present, aligned, and observable",
            ),
            LifecycleStatus::Missing => LifecycleProjection::missing_adopt(
                "runtime principal or required owned paths are missing",
                Some("cynic bootstrap host principal".to_string()),
            ),
            LifecycleStatus::PrincipalMismatch => LifecycleProjection::principal_mismatch_adopt(
                "runtime principal exists but home alignment or path ownership still mismatches policy",
                Some("cynic bootstrap host principal".to_string()),
            ),
            LifecycleStatus::ConnectionRequired => {
                LifecycleProjection::connection_required_observe(
<<<<<<< HEAD
                    "path ownership cannot be proven inside the current identity namespace; observe through a host-authoritative adapter",
=======
                    "principal lifecycle does not model a connection boundary",
>>>>>>> github/main
                )
            }
        };
        PrincipalStatusReport {
            status,
            lifecycle_status,
            lifecycle_disposition: projection.disposition,
            next_boundary_kind: projection.next_boundary_kind,
            next_boundary_reason: projection.next_boundary_reason,
            next_command: projection.next_command,
            user_present: observed.user_present,
            group_present: observed.group_present,
            home_matches,
            missing_state_dirs,
            misowned_paths,
<<<<<<< HEAD
            ownership_unknown_paths,
=======
>>>>>>> github/main
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
        let mut operations = vec![
            HostOperation::EnsureSystemGroup {
                group: self.runtime_group.clone(),
            },
            HostOperation::EnsureSystemUser {
                user: self.runtime_user.clone(),
                group: self.runtime_group.clone(),
                home: self.home_dir.clone(),
                shell: PrincipalLoginShell::NoLogin,
            },
        ];
        operations.extend(self.managed_paths().into_iter().map(|path| {
            HostOperation::EnsureOwnedDir {
                path,
                user: self.runtime_user.clone(),
                group: self.runtime_group.clone(),
                mode: MODE_0755.unix,
            }
        }));
        operations
    }

    pub fn managed_paths(&self) -> Vec<String> {
        let mut paths = vec![self.home_dir.clone(), self.config_dir.clone()];
        paths.extend(self.state_dirs.clone());
        paths.sort();
        paths.dedup();
        paths
    }

    fn install_commands(&self) -> Vec<HostCommandSurface> {
        self.managed_paths()
            .into_iter()
            .map(|path| {
                install_dir_command(
                    "prepare_directory",
                    path,
                    InstallPathPolicy {
                        owner: InstallOwnership {
                            user: Cow::Owned(self.runtime_user.clone()),
                            group: Cow::Owned(self.runtime_group.clone()),
                        },
                        mode: MODE_0755,
                    },
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{PrincipalCommandContext, PrincipalObservedState, PrincipalPolicy};
    use crate::command_surface::render_command;
<<<<<<< HEAD
    use crate::domain::shared::{LifecycleStatus, SurfaceStatus};
=======
    use crate::domain::shared::SurfaceStatus;
>>>>>>> github/main
    use crate::runtime_authority::default_runtime_principal_layout;

    fn context() -> PrincipalCommandContext {
        PrincipalCommandContext {
            getent_bin: "/portable/bin/getent".to_string(),
        }
    }

    fn policy() -> PrincipalPolicy {
        let principal = default_runtime_principal_layout();
        PrincipalPolicy {
            runtime_user: principal.runtime_user,
            runtime_group: principal.runtime_group,
            home_dir: principal.home_dir.display().to_string(),
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
            owned_paths: policy().managed_paths(),
<<<<<<< HEAD
            ownership_unknown_paths: Vec::new(),
=======
>>>>>>> github/main
        });

        assert_eq!(report.status, SurfaceStatus::Blocked);
    }
<<<<<<< HEAD

    #[test]
    fn unknown_namespace_ownership_never_becomes_principal_mismatch() {
        let policy = policy();
        let unknown_paths = policy.managed_paths();
        let report = policy.status(&PrincipalObservedState {
            user_present: true,
            group_present: true,
            observed_home_dir: Some(policy.home_dir.clone()),
            present_state_dirs: policy.state_dirs.clone(),
            owned_paths: Vec::new(),
            ownership_unknown_paths: unknown_paths.clone(),
        });

        assert_eq!(report.status, SurfaceStatus::Blocked);
        assert_eq!(report.lifecycle_status, LifecycleStatus::ConnectionRequired);
        assert!(report.misowned_paths.is_empty());
        assert_eq!(report.ownership_unknown_paths, unknown_paths);
        assert!(report.next_command.is_none());
    }
=======
>>>>>>> github/main
}
