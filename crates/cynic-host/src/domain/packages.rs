use serde::{Deserialize, Serialize};

use crate::command_surface::{HostCommandSurface, argv_command, expectation, install_dir_command};
use crate::domain::shared::{
    BoundaryKind, HostSubstrateDomain, LifecycleDisposition, LifecycleProjection, LifecycleStatus,
    SubstrateBundleFile, SubstrateDomainContract, SubstrateEvidence, SurfaceContract,
    SurfaceStatus,
};
use crate::install_contract::{MODE_0644, MODE_0755, root_owned_path};
use crate::ports::HostOperation;
use crate::runtime_layout::{PackageManagerKind, current_runtime_layout};

const DEFAULT_OS_RELEASE_PATH: &str = "/etc/os-release";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackagePolicy {
    pub manager: PackageManagerKind,
    pub policy_dir: String,
    pub source_projection_path: String,
    pub approved_sources: Vec<String>,
    pub bootstrap_packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageStatusReport {
    pub status: SurfaceStatus,
    pub lifecycle_status: LifecycleStatus,
    pub lifecycle_disposition: LifecycleDisposition,
    pub next_boundary_kind: BoundaryKind,
    pub next_boundary_reason: String,
    pub next_command: Option<String>,
    pub approved_sources_declared: bool,
    pub installed_packages: Vec<String>,
    pub missing_packages: Vec<String>,
    pub evidences: Vec<SubstrateEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageBundleReport {
    pub status: SurfaceStatus,
    pub files: Vec<SubstrateBundleFile>,
    pub commands: Vec<HostCommandSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageActivationPlan {
    pub status: SurfaceStatus,
    pub preflight: Vec<HostCommandSurface>,
    pub install_commands: Vec<HostCommandSurface>,
    pub postflight: Vec<HostCommandSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageApplyReport {
    pub status: SurfaceStatus,
    pub dry_run: bool,
    pub commands: Vec<HostCommandSurface>,
    pub proofs_required: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageObservedState {
    pub installed_packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageCommandContext {
    pub package_query_bin: String,
    pub package_install_bin: String,
    pub apk_bin: String,
    pub dnf_bin: String,
    pub rpm_bin: String,
    pub which_bin: String,
    pub os_release_path: String,
}

impl PackageCommandContext {
    pub fn from_current_runtime() -> Self {
        let layout = current_runtime_layout();
        Self {
            package_query_bin: layout.package_query_bin(),
            package_install_bin: layout.package_install_bin(),
            apk_bin: layout.apk_bin(),
            dnf_bin: layout.dnf_bin(),
            rpm_bin: layout.rpm_bin(),
            which_bin: layout.which_bin(),
            os_release_path: DEFAULT_OS_RELEASE_PATH.to_string(),
        }
    }
}

impl PackagePolicy {
    pub fn contract(&self) -> SubstrateDomainContract {
        SubstrateDomainContract {
            domain: HostSubstrateDomain::Packages,
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
                id: "package_policy_declared".to_string(),
                status: SurfaceStatus::Planned,
                detail: format!(
                    "approved_sources={} bootstrap_packages={}",
                    self.approved_sources.join(","),
                    self.bootstrap_packages.join(",")
                ),
            }],
        }
    }

    pub fn bundle(&self) -> PackageBundleReport {
        self.bundle_with_context(&PackageCommandContext::from_current_runtime())
    }

    pub fn bundle_with_context(&self, context: &PackageCommandContext) -> PackageBundleReport {
        PackageBundleReport {
            status: SurfaceStatus::Planned,
            files: vec![SubstrateBundleFile {
                logical_name: "package-source-policy".to_string(),
                install_path: self.source_projection_path.clone(),
                mode: MODE_0644.octal.to_string(),
            }],
            commands: self.install_commands(context),
        }
    }

    pub fn activation_plan(&self) -> PackageActivationPlan {
        self.activation_plan_with_context(&PackageCommandContext::from_current_runtime())
    }

    pub fn activation_plan_with_context(
        &self,
        context: &PackageCommandContext,
    ) -> PackageActivationPlan {
        let first_source = self
            .approved_sources
            .first()
            .cloned()
            .unwrap_or_else(|| "<missing-approved-source>".to_string());
        PackageActivationPlan {
            status: SurfaceStatus::Planned,
            preflight: vec![
                argv_command(
                    "check_os_release",
                    vec![
                        "test".to_string(),
                        "-f".to_string(),
                        context.os_release_path.clone(),
                    ],
                ),
                expectation(
                    "selected_source",
                    format!("approved package source selected: {first_source}"),
                ),
            ],
            install_commands: self.install_commands(context),
            postflight: self
                .bootstrap_packages
                .iter()
                .map(|pkg| {
                    argv_command(
                        "package_presence",
                        package_query_argv(self.manager, pkg, context),
                    )
                })
                .collect(),
        }
    }

    pub fn apply_report(&self, dry_run: bool) -> PackageApplyReport {
        self.apply_report_with_context(dry_run, &PackageCommandContext::from_current_runtime())
    }

    pub fn apply_report_with_context(
        &self,
        dry_run: bool,
        context: &PackageCommandContext,
    ) -> PackageApplyReport {
        PackageApplyReport {
            status: SurfaceStatus::Planned,
            dry_run,
            commands: self.install_commands(context),
            proofs_required: self
                .bootstrap_packages
                .iter()
                .map(|pkg| format!("package_present:{pkg}"))
                .collect(),
        }
    }

    pub fn status(&self, observed: &PackageObservedState) -> PackageStatusReport {
        let missing_packages = self
            .bootstrap_packages
            .iter()
            .filter(|pkg| !observed.installed_packages.contains(*pkg))
            .cloned()
            .collect::<Vec<_>>();
        let approved_sources_declared = !self.approved_sources.is_empty();
        let status = if approved_sources_declared && missing_packages.is_empty() {
            SurfaceStatus::Active
        } else if approved_sources_declared {
            SurfaceStatus::Blocked
        } else {
            SurfaceStatus::Degraded
        };
        let lifecycle_status = if !approved_sources_declared {
            LifecycleStatus::Missing
        } else if !missing_packages.is_empty() {
            LifecycleStatus::PrincipalMismatch
        } else {
            LifecycleStatus::Ready
        };
        let projection = match lifecycle_status {
            LifecycleStatus::Ready => LifecycleProjection::ready_observe(
                "approved package sources are declared and bootstrap packages are present",
            ),
            LifecycleStatus::Missing => LifecycleProjection::missing_adopt(
                "approved package source projection is not yet declared",
                Some("cynic host bootstrap packages --json".to_string()),
            ),
            LifecycleStatus::PrincipalMismatch => LifecycleProjection::principal_mismatch_adopt(
                "approved package sources are declared but one or more bootstrap packages are still missing",
                Some("cynic host bootstrap packages --json".to_string()),
            ),
            LifecycleStatus::ConnectionRequired => {
                LifecycleProjection::connection_required_observe(
                    "package lifecycle does not model a connection boundary",
                )
            }
        };
        PackageStatusReport {
            status,
            lifecycle_status,
            lifecycle_disposition: projection.disposition,
            next_boundary_kind: projection.next_boundary_kind,
            next_boundary_reason: projection.next_boundary_reason,
            next_command: projection.next_command,
            approved_sources_declared,
            installed_packages: observed.installed_packages.clone(),
            missing_packages,
            evidences: vec![
                SubstrateEvidence {
                    id: "package_sources_declared".to_string(),
                    status: if approved_sources_declared {
                        SurfaceStatus::Active
                    } else {
                        SurfaceStatus::Blocked
                    },
                    detail: self.approved_sources.join(","),
                },
                SubstrateEvidence {
                    id: "bootstrap_packages_expected".to_string(),
                    status: SurfaceStatus::Planned,
                    detail: self.bootstrap_packages.join(","),
                },
            ],
        }
    }

    fn install_commands(&self, context: &PackageCommandContext) -> Vec<HostCommandSurface> {
        let mut commands = vec![
            install_dir_command(
                "prepare_directory",
                self.policy_dir.clone(),
                root_owned_path(MODE_0755),
            ),
            expectation(
                "render_package_source_projection",
                format!(
                    "render {} from approved package sources",
                    self.source_projection_path
                ),
            ),
        ];
        let install_argv = package_install_argv(self.manager, &self.bootstrap_packages, context);
        if install_argv.is_empty() {
            commands.push(expectation(
                "install_bootstrap_packages",
                format!(
                    "install bootstrap packages from approved sources: {}",
                    self.bootstrap_packages.join(",")
                ),
            ));
        } else {
            commands.push(argv_command("install_bootstrap_packages", install_argv));
        }
        commands
    }

    pub fn apply_operations(&self) -> Vec<HostOperation> {
        vec![
            HostOperation::EnsureDir {
                path: self.policy_dir.clone(),
                mode: MODE_0755.unix,
            },
            HostOperation::WritePackageSourceProjection {
                path: self.source_projection_path.clone(),
                approved_sources: self.approved_sources.clone(),
            },
            HostOperation::InstallBootstrapPackages {
                approved_sources: self.approved_sources.clone(),
                packages: self.bootstrap_packages.clone(),
            },
        ]
    }
}

fn package_query_argv(
    manager: PackageManagerKind,
    package: &str,
    context: &PackageCommandContext,
) -> Vec<String> {
    match manager {
        PackageManagerKind::Apt => vec![
            context.package_query_bin.clone(),
            "-s".to_string(),
            package.to_string(),
        ],
        PackageManagerKind::Apk => vec![
            context.apk_bin.clone(),
            "info".to_string(),
            "-e".to_string(),
            package.to_string(),
        ],
        PackageManagerKind::Dnf | PackageManagerKind::Rpm => {
            vec![
                context.rpm_bin.clone(),
                "-q".to_string(),
                package.to_string(),
            ]
        }
        PackageManagerKind::Winget => vec![
            context.package_query_bin.clone(),
            "list".to_string(),
            "--exact".to_string(),
            "--id".to_string(),
            package.to_string(),
        ],
        PackageManagerKind::Choco => vec![
            context.package_query_bin.clone(),
            "list".to_string(),
            "--exact".to_string(),
            package.to_string(),
        ],
        PackageManagerKind::Unknown => vec![context.which_bin.clone(), package.to_string()],
    }
}

fn package_install_argv(
    manager: PackageManagerKind,
    packages: &[String],
    context: &PackageCommandContext,
) -> Vec<String> {
    match manager {
        PackageManagerKind::Apt => {
            let mut argv = vec![
                context.package_install_bin.clone(),
                "install".to_string(),
                "-y".to_string(),
            ];
            argv.extend(packages.iter().cloned());
            argv
        }
        PackageManagerKind::Apk => {
            let mut argv = vec![
                context.apk_bin.clone(),
                "add".to_string(),
                "--no-cache".to_string(),
            ];
            argv.extend(packages.iter().cloned());
            argv
        }
        PackageManagerKind::Dnf => {
            let mut argv = vec![
                context.dnf_bin.clone(),
                "install".to_string(),
                "-y".to_string(),
            ];
            argv.extend(packages.iter().cloned());
            argv
        }
        PackageManagerKind::Winget => {
            let mut argv = vec![
                context.package_install_bin.clone(),
                "install".to_string(),
                "--exact".to_string(),
                "--accept-source-agreements".to_string(),
                "--accept-package-agreements".to_string(),
                "--id".to_string(),
            ];
            argv.extend(packages.iter().cloned());
            argv
        }
        PackageManagerKind::Choco => {
            let mut argv = vec![
                context.package_install_bin.clone(),
                "install".to_string(),
                "-y".to_string(),
            ];
            argv.extend(packages.iter().cloned());
            argv
        }
        PackageManagerKind::Rpm | PackageManagerKind::Unknown => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{PackageCommandContext, PackagePolicy};
    use crate::command_surface::render_command;
    use crate::runtime_layout::PackageManagerKind;

    fn context() -> PackageCommandContext {
        PackageCommandContext {
            package_query_bin: "/portable/bin/dpkg".to_string(),
            package_install_bin: "/portable/bin/apt-get".to_string(),
            apk_bin: "/portable/bin/apk".to_string(),
            dnf_bin: "/portable/bin/dnf".to_string(),
            rpm_bin: "/portable/bin/rpm".to_string(),
            which_bin: "/portable/bin/which".to_string(),
            os_release_path: "/portable/etc/os-release".to_string(),
        }
    }

    fn policy(manager: PackageManagerKind) -> PackagePolicy {
        PackagePolicy {
            manager,
            policy_dir: "/etc/cynic".to_string(),
            source_projection_path: "/etc/cynic/package-sources.json".to_string(),
            approved_sources: vec!["distribution_package".to_string()],
            bootstrap_packages: vec!["coredns".to_string(), "wireguard-tools".to_string()],
        }
    }

    #[test]
    fn apk_policy_renders_apk_install_and_presence_commands() {
        let package_policy = policy(PackageManagerKind::Apk);
        let activation = package_policy.activation_plan_with_context(&context());
        let rendered = activation
            .preflight
            .iter()
            .chain(activation.install_commands.iter())
            .chain(activation.postflight.iter())
            .map(render_command)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("test -f /portable/etc/os-release"));
        assert!(rendered.contains("/portable/bin/apk add --no-cache coredns wireguard-tools"));
        assert!(rendered.contains("/portable/bin/apk info -e coredns"));
        assert!(!rendered.contains("apt-get"));
        assert!(!rendered.contains("dpkg"));
    }

    #[test]
    fn apt_policy_renders_apt_and_dpkg_commands() {
        let package_policy = policy(PackageManagerKind::Apt);
        let activation = package_policy.activation_plan_with_context(&context());
        let rendered = activation
            .install_commands
            .iter()
            .chain(activation.postflight.iter())
            .map(render_command)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("/portable/bin/apt-get install -y coredns wireguard-tools"));
        assert!(rendered.contains("/portable/bin/dpkg -s coredns"));
    }

    #[test]
    fn apply_report_uses_injected_package_command_context() {
        let package_policy = policy(PackageManagerKind::Dnf);
        let report = package_policy.apply_report_with_context(true, &context());
        let rendered = report
            .commands
            .iter()
            .map(render_command)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("/portable/bin/dnf install -y coredns wireguard-tools"));
    }
}
