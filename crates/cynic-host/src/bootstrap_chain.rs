use serde::Serialize;
use serde_json::Value;

use crate::adapters::local_observation::LocalHostObservationAdapter;
use crate::bridge;
use crate::command_surface::{HostCommandSurface, render_command, shell_command};
use crate::domain::bootstrap::{build_bootstrap_plan, build_bootstrap_status};
use crate::domain::shared::{
    BoundaryKind, LifecycleDisposition, LifecycleProjection, LifecycleStatus, SurfaceStatus,
};
use crate::ports::{
    HostPackageObservationPort, HostPrincipalObservationPort, HostPrivilegeObservationPort,
    HostServiceObservationPort,
};
use crate::profiles::HostProfile;
use crate::registry::HostProfileResolution;
use crate::runtime_context::{HostRuntimeContext, observe_host_runtime_context_with_observer};
use crate::runtime_layout::current_runtime_layout;
use crate::usecases::{
    build_host_activation_plan_with_resolution, build_host_apply_report_with_resolution,
    build_host_bundle_with_resolution, observe_host_with_resolution,
};

#[derive(Debug, Serialize)]
pub struct HostBootstrapChainReport {
    pub schema: &'static str,
    pub status: SurfaceStatus,
    pub lifecycle_status: LifecycleStatus,
    pub lifecycle_disposition: LifecycleDisposition,
    pub next_boundary_kind: BoundaryKind,
    pub next_boundary_reason: String,
    pub next_command: Option<String>,
    pub host_profile: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub stages: Vec<HostBootstrapStage>,
    pub host_context: Value,
    pub host_bundle: Value,
    pub host_plan: Value,
    pub host_apply_report: Value,
    pub bridge_status: Value,
    pub bridge_plan: Value,
    pub bootstrap_status: Value,
    pub bootstrap_plan: Value,
}

#[derive(Debug, Serialize)]
pub struct HostBootstrapStage {
    pub order: u8,
    pub id: &'static str,
    pub layer: &'static str,
    pub status: SurfaceStatus,
    pub objective: &'static str,
    pub canonical_surface: &'static str,
    pub commands: Vec<HostCommandSurface>,
}

pub fn build_bootstrap_chain_report(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
) -> Result<HostBootstrapChainReport, Box<dyn std::error::Error>> {
    build_bootstrap_chain_report_with_observer(profile, resolution, &LocalHostObservationAdapter)
}

pub fn build_bootstrap_chain_report_with_observer(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
    observed_adapter: &(
         impl HostPrincipalObservationPort
         + HostPrivilegeObservationPort
         + HostPackageObservationPort
         + HostServiceObservationPort
     ),
) -> Result<HostBootstrapChainReport, Box<dyn std::error::Error>> {
    let host_context =
        observe_host_runtime_context_with_observer(profile, resolution.clone(), observed_adapter);
    let observed = observe_host_with_resolution(&profile, observed_adapter, resolution.clone());
    let host_bundle = build_host_bundle_with_resolution(&profile, resolution.clone());
    let host_plan =
        build_host_activation_plan_with_resolution(&profile, &observed, resolution.clone());
    let host_apply_report =
        build_host_apply_report_with_resolution(&profile, &observed, true, resolution.clone());
    let bridge_status = bridge::bridge_status();
    let runtime_layout = current_runtime_layout();
    let bridge_target = resolution
        .as_ref()
        .and_then(|item| item.matched_node.clone())
        .or_else(|| {
            resolution
                .as_ref()
                .map(|item| item.hostname_observed.clone())
        })
        .unwrap_or_else(|| "local-host".to_string());
    let bridge_spec =
        bridge::privilege_bridge_install_spec(&runtime_layout.host_exec_bundle_dir, bridge_target);
    let bridge_plan = bridge::privilege_bridge_plan(&bridge_spec);
    let bootstrap_status = build_bootstrap_status(
        bridge_status.status == "active",
        host_substrate_active(&host_context),
    );
    let bootstrap_plan = build_bootstrap_plan(&bootstrap_status);

    let stages = vec![
        HostBootstrapStage {
            order: 1,
            id: "host_substrate_bundle",
            layer: "os_baseline",
            status: host_bundle.status,
            objective: "render the canonical host baseline substrate bundle",
            canonical_surface: "cynic bootstrap host bundle",
            commands: vec![bootstrap_command(
                "bundle",
                "bootstrap host bundle --registry <registry>",
                profile.name(),
                &resolution,
            )],
        },
        HostBootstrapStage {
            order: 2,
            id: "host_substrate_plan",
            layer: "os_baseline",
            status: host_plan.status,
            objective: "plan host baseline activation for principal, privilege, packages and services",
            canonical_surface: "cynic bootstrap host plan",
            commands: vec![
                bootstrap_command(
                    "observe",
                    "bootstrap host observe --registry <registry>",
                    profile.name(),
                    &resolution,
                ),
                bootstrap_command(
                    "plan",
                    "bootstrap host plan --registry <registry>",
                    profile.name(),
                    &resolution,
                ),
                bootstrap_command(
                    "apply_dry_run",
                    "bootstrap host apply --registry <registry> --dry-run",
                    profile.name(),
                    &resolution,
                ),
            ],
        },
        HostBootstrapStage {
            order: 3,
            id: "host_bridge",
            layer: "privilege_bridge",
            status: if bridge_status.status == "active" {
                SurfaceStatus::Active
            } else {
                SurfaceStatus::Blocked
            },
            objective: "install the bounded host-exec helper and sudoers bridge once",
            canonical_surface: "cynic bootstrap bridge plan",
            commands: vec![
                shell_command(
                    "bundle",
                    format!(
                        "cynic bootstrap bridge bundle --output-dir {}",
                        runtime_layout.host_exec_bundle_dir
                    ),
                ),
                shell_command("plan", "cynic bootstrap bridge plan"),
                shell_command("status", "cynic bootstrap bridge status"),
            ],
        },
        HostBootstrapStage {
            order: 4,
            id: "bootstrap_proof",
            layer: "bootstrap_runtime_boundary",
            status: bootstrap_status.status,
            objective: "prove runtime no longer depends on a human sudo gesture",
            canonical_surface: "cynic bootstrap chain",
            commands: vec![bootstrap_command(
                "chain",
                "bootstrap chain --registry <registry>",
                profile.name(),
                &resolution,
            )],
        },
    ];

    let status = if stages
        .iter()
        .all(|stage| stage.status == SurfaceStatus::Active)
    {
        SurfaceStatus::Active
    } else {
        SurfaceStatus::Blocked
    };
    let first_blocked = stages
        .iter()
        .find(|stage| stage.status != SurfaceStatus::Active);
    let lifecycle_status = if status == SurfaceStatus::Active {
        LifecycleStatus::Ready
    } else if matches!(first_blocked.map(|stage| stage.id), Some("host_bridge")) {
        LifecycleStatus::Missing
    } else {
        LifecycleStatus::PrincipalMismatch
    };
    let projection = match lifecycle_status {
        LifecycleStatus::Ready => LifecycleProjection::ready_observe(
            "bootstrap chain stages are active through the runtime boundary proof",
        ),
        LifecycleStatus::Missing => LifecycleProjection::missing_adopt(
            "bootstrap chain is blocked at bridge installation or activation",
            first_blocked
                .and_then(|stage| stage.commands.first())
                .map(render_command),
        ),
        LifecycleStatus::PrincipalMismatch => LifecycleProjection::principal_mismatch_adopt(
            "bootstrap chain is partially satisfied but at least one declared stage remains blocked",
            first_blocked
                .and_then(|stage| stage.commands.first())
                .map(render_command),
        ),
        LifecycleStatus::ConnectionRequired => LifecycleProjection::connection_required_observe(
            "bootstrap chain does not model a connection boundary",
        ),
    };

    Ok(HostBootstrapChainReport {
        schema: "cynic.host_bootstrap.chain.v1",
        status,
        lifecycle_status,
        lifecycle_disposition: projection.disposition,
        next_boundary_kind: projection.next_boundary_kind,
        next_boundary_reason: projection.next_boundary_reason,
        next_command: projection.next_command,
        host_profile: profile.name(),
        host_resolution: resolution,
        stages,
        host_context: serde_json::to_value(&host_context)?,
        host_bundle: serde_json::to_value(&host_bundle)?,
        host_plan: serde_json::to_value(&host_plan)?,
        host_apply_report: serde_json::to_value(&host_apply_report)?,
        bridge_status: serde_json::to_value(&bridge_status)?,
        bridge_plan: serde_json::to_value(&bridge_plan)?,
        bootstrap_status: serde_json::to_value(&bootstrap_status)?,
        bootstrap_plan: serde_json::to_value(&bootstrap_plan)?,
    })
}

fn host_substrate_active(context: &HostRuntimeContext) -> bool {
    let reports = &context.host_substrate;
    reports.principal.status == SurfaceStatus::Active
        && reports.privilege.status == SurfaceStatus::Active
        && reports.packages.status == SurfaceStatus::Active
        && reports.services.status == SurfaceStatus::Active
}

fn bootstrap_command(
    role: &'static str,
    suffix: &str,
    explicit_profile: &str,
    resolution: &Option<HostProfileResolution>,
) -> HostCommandSurface {
    let base = "cynic";
    let command = match resolution {
        Some(resolution) if resolution.match_source == "explicit_profile" => {
            format!("{base} {suffix} --profile {explicit_profile}")
        }
        Some(resolution) if resolution.matched_node.is_some() => format!(
            "{base} {suffix} --node {}{}",
            resolution.matched_node.as_deref().unwrap_or("unknown"),
            resolution
                .registry_path
                .as_ref()
                .map(|path| format!(" --registry {path}"))
                .unwrap_or_default()
        ),
        Some(resolution) => format!(
            "{base} {suffix}{}",
            resolution
                .registry_path
                .as_ref()
                .map(|path| format!(" --registry {path}"))
                .unwrap_or_default()
        ),
        None => format!("{base} {suffix}"),
    };
    shell_command(role, command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::packages::PackageObservedState;
    use crate::domain::principal::PrincipalObservedState;
    use crate::domain::privilege::PrivilegeObservedState;
    use crate::domain::services::{ServiceObservedState, ServiceObservedUnit};
    use crate::ports::{
        HostPackageObservationPort, HostPrincipalObservationPort, HostPrivilegeObservationPort,
        HostServiceObservationPort,
    };

    struct FakeHostObservationAdapter;

    impl HostPrincipalObservationPort for FakeHostObservationAdapter {
        fn observe_principal(
            &self,
            policy: &crate::domain::principal::PrincipalPolicy,
        ) -> PrincipalObservedState {
            PrincipalObservedState {
                user_present: true,
                group_present: true,
                observed_home_dir: Some(policy.home_dir.clone()),
                present_state_dirs: policy.state_dirs.clone(),
                owned_paths: policy.managed_paths(),
                ownership_unknown_paths: Vec::new(),
            }
        }
    }

    impl HostPrivilegeObservationPort for FakeHostObservationAdapter {
        fn observe_privilege(
            &self,
            _policy: &crate::domain::privilege::PrivilegePolicy,
        ) -> PrivilegeObservedState {
            PrivilegeObservedState {
                helper_dir_present: true,
                sudoers_projection_present: true,
                denial_proof_ready: true,
            }
        }
    }

    impl HostPackageObservationPort for FakeHostObservationAdapter {
        fn observe_packages(
            &self,
            policy: &crate::domain::packages::PackagePolicy,
        ) -> PackageObservedState {
            PackageObservedState {
                installed_packages: policy.bootstrap_packages.clone(),
            }
        }
    }

    impl HostServiceObservationPort for FakeHostObservationAdapter {
        fn observe_services(
            &self,
            policy: &crate::domain::services::ServicePolicy,
        ) -> ServiceObservedState {
            ServiceObservedState {
                units: policy
                    .units
                    .iter()
                    .map(|unit| ServiceObservedUnit {
                        unit_name: unit.unit_name.clone(),
                        present: true,
                        enabled: true,
                        active: true,
                    })
                    .collect(),
            }
        }
    }

    #[test]
    fn bootstrap_chain_report_has_stable_shape() {
        let report = build_bootstrap_chain_report_with_observer(
            HostProfile::NetworkNode,
            None,
            &FakeHostObservationAdapter,
        )
        .expect("report");
        assert_eq!(report.schema, "cynic.host_bootstrap.chain.v1");
        assert_eq!(report.stages.len(), 4);
        assert_eq!(
            report.stages.first().map(|stage| stage.id),
            Some("host_substrate_bundle")
        );
        assert_eq!(
            report.stages.last().map(|stage| stage.id),
            Some("bootstrap_proof")
        );
        match report.status {
            SurfaceStatus::Active => {
                assert_eq!(report.lifecycle_status, LifecycleStatus::Ready);
                assert_eq!(report.next_boundary_kind, BoundaryKind::Observe);
                assert!(report.next_command.is_none());
            }
            SurfaceStatus::Blocked => {
                assert_ne!(report.lifecycle_status, LifecycleStatus::Ready);
                assert_ne!(report.next_boundary_kind, BoundaryKind::Observe);
                assert!(report.next_command.is_some());
            }
            other => panic!("unexpected bootstrap chain status: {other:?}"),
        }
        assert!(!report.next_boundary_reason.is_empty());
    }
}
