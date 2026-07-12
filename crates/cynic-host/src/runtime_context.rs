use serde::Serialize;

use crate::adapters::local_observation::LocalHostObservationAdapter;
use crate::bridge;
use crate::domain::bootstrap::{
    BootstrapPlanReport, BootstrapStatusReport, build_bootstrap_plan, build_bootstrap_status,
};
use crate::domain::shared::SurfaceStatus;
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
    pub host_substrate: HostObservationReports,
    pub host_substrate_plan: HostActivationPlanReport,
    pub host_bootstrap: BootstrapStatusReport,
    pub host_bootstrap_plan: BootstrapPlanReport,
}

pub fn observe_current_host_runtime_context() -> HostRuntimeContext {
    let profile = resolve_current_host_profile(None).unwrap_or(HostProfile::NetworkNode);
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
        host_substrate,
        host_substrate_plan,
        host_bootstrap,
        host_bootstrap_plan,
    }
}
