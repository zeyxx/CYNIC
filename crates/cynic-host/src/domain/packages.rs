use serde::{Deserialize, Serialize};

use crate::command_surface::{
    HostCommandSurface, argv_command, expectation, install_dir_command, package_presence_command,
};
use crate::domain::shared::{
    HostSubstrateDomain, SubstrateBundleFile, SubstrateDomainContract, SubstrateEvidence,
    SurfaceContract, SurfaceStatus,
};
use crate::install_contract::{MODE_0644, MODE_0755, root_owned_path};
use crate::ports::HostOperation;
use crate::runtime_layout::current_runtime_layout;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackagePolicy {
    pub policy_dir: String,
    pub source_projection_path: String,
    pub approved_sources: Vec<String>,
    pub bootstrap_packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageStatusReport {
    pub status: SurfaceStatus,
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
        PackageBundleReport {
            status: SurfaceStatus::Planned,
            files: vec![SubstrateBundleFile {
                logical_name: "package-source-policy".to_string(),
                install_path: self.source_projection_path.clone(),
                mode: MODE_0644.octal.to_string(),
            }],
            commands: self.install_commands(),
        }
    }

    pub fn activation_plan(&self) -> PackageActivationPlan {
        let runtime_layout = current_runtime_layout();
        let capabilities = runtime_layout.capabilities();
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
                        "/etc/os-release".to_string(),
                    ],
                ),
                expectation(
                    "selected_source",
                    format!("approved package source selected: {first_source}"),
                ),
            ],
            install_commands: self.install_commands(),
            postflight: self
                .bootstrap_packages
                .iter()
                .map(|pkg| {
                    package_presence_command(
                        "package_presence",
                        pkg.clone(),
                        capabilities.package_manager.query.clone(),
                        capabilities.package_manager.rpm_query.clone(),
                    )
                })
                .collect(),
        }
    }

    pub fn apply_report(&self, dry_run: bool) -> PackageApplyReport {
        PackageApplyReport {
            status: SurfaceStatus::Planned,
            dry_run,
            commands: self.install_commands(),
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
        PackageStatusReport {
            status,
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

    fn install_commands(&self) -> Vec<HostCommandSurface> {
        vec![
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
            expectation(
                "install_bootstrap_packages",
                format!(
                    "install bootstrap packages from approved sources: {}",
                    self.bootstrap_packages.join(",")
                ),
            ),
        ]
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
