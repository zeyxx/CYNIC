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

const DEFAULT_DENIED_SHELL_BIN: &str = "/bin/sh";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivilegeCommandContext {
    pub sudo_bin: String,
    pub route_bin: String,
    pub service_manager_bin: String,
    pub denied_shell_bin: String,
}

impl PrivilegeCommandContext {
    pub fn from_current_runtime() -> Self {
        let runtime_layout = current_runtime_layout();
        let capabilities = runtime_layout.capabilities();
        Self {
            sudo_bin: runtime_layout.sudo_bin(),
            route_bin: capabilities.route_tool.command,
            service_manager_bin: capabilities.service_manager.command,
            denied_shell_bin: DEFAULT_DENIED_SHELL_BIN.to_string(),
        }
    }
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
        self.activation_plan_with_context(&PrivilegeCommandContext::from_current_runtime())
    }

    pub fn activation_plan_with_context(
        &self,
        context: &PrivilegeCommandContext,
    ) -> PrivilegeActivationPlan {
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
                        context.sudo_bin.clone(),
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
                        context.sudo_bin.clone(),
                        "-n".to_string(),
                        context.route_bin.clone(),
                        "route".to_string(),
                        "show".to_string(),
                    ],
                ),
                argv_command(
                    "deny_service_status",
                    vec![
                        context.sudo_bin.clone(),
                        "-n".to_string(),
                        context.service_manager_bin.clone(),
                        "status".to_string(),
                        "cynic-node.service".to_string(),
                    ],
                ),
                argv_command(
                    "deny_shell",
                    vec![
                        context.sudo_bin.clone(),
                        "-n".to_string(),
                        context.denied_shell_bin.clone(),
                        "-c".to_string(),
                        "true".to_string(),
                    ],
                ),
            ],
        }
    }

    pub fn apply_report(&self, dry_run: bool) -> PrivilegeApplyReport {
        self.apply_report_with_context(dry_run, &PrivilegeCommandContext::from_current_runtime())
    }

    pub fn apply_report_with_context(
        &self,
        dry_run: bool,
        context: &PrivilegeCommandContext,
    ) -> PrivilegeApplyReport {
        let activation_plan = self.activation_plan_with_context(context);
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

#[cfg(test)]
mod tests {
    use super::{PrivilegeCommandContext, PrivilegeObservedState, PrivilegePolicy};
    use crate::command_surface::render_command;
    use crate::domain::shared::SurfaceStatus;

    fn context() -> PrivilegeCommandContext {
        PrivilegeCommandContext {
            sudo_bin: "/portable/bin/sudo".to_string(),
            route_bin: "/portable/bin/ip".to_string(),
            service_manager_bin: "/portable/bin/rc-service".to_string(),
            denied_shell_bin: "/portable/bin/sh".to_string(),
        }
    }

    fn policy() -> PrivilegePolicy {
        PrivilegePolicy {
            sudoers_subject: "cynic".to_string(),
            helper_dir: "/opt/cynic/libexec".to_string(),
            config_dir: "/opt/cynic/etc".to_string(),
            sudoers_dir: "/opt/cynic/sudoers.d".to_string(),
            denial_proof_required: true,
            dedicated_helpers: vec!["cynic-host-exec".to_string()],
        }
    }

    #[test]
    fn activation_plan_uses_injected_privilege_command_context() {
        let activation = policy().activation_plan_with_context(&context());
        let rendered = activation
            .postflight
            .iter()
            .chain(activation.denial_checks.iter())
            .map(render_command)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("/portable/bin/sudo -n -u cynic true"));
        assert!(rendered.contains("/portable/bin/sudo -n /portable/bin/ip route show"));
        assert!(
            rendered.contains(
                "/portable/bin/sudo -n /portable/bin/rc-service status cynic-node.service"
            )
        );
        assert!(rendered.contains("/portable/bin/sudo -n /portable/bin/sh -c true"));
    }

    #[test]
    fn apply_report_uses_injected_denial_proof_context() {
        let report = policy().apply_report_with_context(true, &context());
        let rendered = report.denial_proofs_required.join("\n");

        assert!(rendered.contains("/portable/bin/ip route show"));
        assert!(rendered.contains("/portable/bin/rc-service status cynic-node.service"));
        assert!(rendered.contains("/portable/bin/sh -c true"));
    }

    #[test]
    fn status_requires_denial_proof_when_enabled() {
        let report = policy().status(&PrivilegeObservedState {
            helper_dir_present: true,
            sudoers_projection_present: true,
            denial_proof_ready: false,
        });

        assert_eq!(report.status, SurfaceStatus::Blocked);
    }
}
