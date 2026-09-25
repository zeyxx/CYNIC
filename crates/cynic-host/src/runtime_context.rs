use serde::Serialize;

use crate::adapters::local_observation::LocalHostObservationAdapter;
use crate::bridge;
use crate::domain::bootstrap::{
    BootstrapPlanReport, BootstrapStatusReport, build_bootstrap_plan, build_bootstrap_status,
};
use crate::domain::shared::{BoundaryKind, LifecycleDisposition, LifecycleStatus, SurfaceStatus};
use crate::ports::{
    HostPackageObservationPort, HostPrincipalObservationPort, HostPrivilegeObservationPort,
    HostServiceObservationPort,
};
use crate::profiles::HostProfile;
use crate::registry::{
    HostProfileResolution, resolve_current_host_profile, resolve_current_host_profile_resolution,
};
use crate::usecases::{
    HostActivationPlanReport, HostObservationReports, build_host_activation_plan_with_resolution,
    observe_host_with_resolution,
};

#[derive(Debug, Serialize)]
pub struct HostRuntimeContext {
    pub host_profile: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub lifecycle_status: LifecycleStatus,
    pub lifecycle_disposition: LifecycleDisposition,
    pub next_boundary_kind: BoundaryKind,
    pub next_boundary_reason: String,
    pub next_command: Option<String>,
    pub host_substrate: HostObservationReports,
    pub host_substrate_plan: HostActivationPlanReport,
    pub host_bootstrap: BootstrapStatusReport,
    pub host_bootstrap_plan: BootstrapPlanReport,
}

pub fn observe_current_host_runtime_context() -> HostRuntimeContext {
<<<<<<< HEAD
    // A missing registry identity is not authority to activate mesh DNS/WireGuard.
    let profile = resolve_current_host_profile(None).unwrap_or(HostProfile::ManagementNode);
=======
    let profile = resolve_current_host_profile(None).unwrap_or(HostProfile::NetworkNode);
>>>>>>> github/main
    let resolution = resolve_current_host_profile_resolution(None).ok();
    observe_host_runtime_context(profile, resolution)
}

pub fn observe_host_runtime_context(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
) -> HostRuntimeContext {
    observe_host_runtime_context_with_observer(profile, resolution, &LocalHostObservationAdapter)
}

pub fn observe_host_runtime_context_with_observer(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
    observed: &(
         impl HostPrincipalObservationPort
         + HostPrivilegeObservationPort
         + HostPackageObservationPort
         + HostServiceObservationPort
     ),
) -> HostRuntimeContext {
    let host_substrate = observe_host_with_resolution(&profile, observed, resolution.clone());
    let host_substrate_plan =
        build_host_activation_plan_with_resolution(&profile, &host_substrate, resolution.clone());
    let host_substrate_active = host_substrate.principal.status == SurfaceStatus::Active
        && host_substrate.privilege.status == SurfaceStatus::Active
        && host_substrate.packages.status == SurfaceStatus::Active
        && host_substrate.services.status == SurfaceStatus::Active;
    let bridge_status = bridge::bridge_status();
    let host_bootstrap =
        build_bootstrap_status(bridge_status.status == "active", host_substrate_active);
    let host_bootstrap_plan = build_bootstrap_plan(&host_bootstrap);

    HostRuntimeContext {
        host_profile: profile.name(),
        host_resolution: resolution,
        lifecycle_status: host_bootstrap.lifecycle_status,
        lifecycle_disposition: host_bootstrap.lifecycle_disposition,
        next_boundary_kind: host_bootstrap.next_boundary_kind,
        next_boundary_reason: host_bootstrap.next_boundary_reason.clone(),
        next_command: host_bootstrap.next_command.clone(),
        host_substrate,
        host_substrate_plan,
        host_bootstrap,
        host_bootstrap_plan,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::packages::PackageObservedState;
    use crate::domain::principal::PrincipalObservedState;
    use crate::domain::privilege::PrivilegeObservedState;
    use crate::domain::services::{ServiceObservedState, ServiceObservedUnit};

    struct ReadyObserver;

    impl HostPrincipalObservationPort for ReadyObserver {
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
<<<<<<< HEAD
                ownership_unknown_paths: Vec::new(),
=======
>>>>>>> github/main
            }
        }
    }

    impl HostPrivilegeObservationPort for ReadyObserver {
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

    impl HostPackageObservationPort for ReadyObserver {
        fn observe_packages(
            &self,
            policy: &crate::domain::packages::PackagePolicy,
        ) -> PackageObservedState {
<<<<<<< HEAD
            crate::domain::packages::promoted_package_state(policy.bootstrap_packages.clone())
=======
            PackageObservedState {
                installed_packages: policy.bootstrap_packages.clone(),
            }
>>>>>>> github/main
        }
    }

    impl HostServiceObservationPort for ReadyObserver {
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
    fn runtime_context_projects_top_level_lifecycle_from_bootstrap() {
        let report = observe_host_runtime_context_with_observer(
            HostProfile::NetworkNode,
            None,
            &ReadyObserver,
        );

        assert_eq!(
            report.lifecycle_status,
            report.host_bootstrap.lifecycle_status
        );
        assert_eq!(
            report.lifecycle_disposition,
            report.host_bootstrap.lifecycle_disposition
        );
        assert_eq!(
            report.next_boundary_kind,
            report.host_bootstrap.next_boundary_kind
        );
        assert_eq!(report.next_command, report.host_bootstrap.next_command);
    }
}
