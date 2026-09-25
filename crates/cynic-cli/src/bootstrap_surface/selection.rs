use std::env;

use clap::Args;
use cynic_host::domain::shared::SurfaceStatus;
use cynic_host::profiles::HostProfile;
use cynic_host::registry::{
    HostProfileResolution, resolve_current_host_profile, resolve_current_host_profile_resolution,
    resolve_host_profile, resolve_host_profile_resolution,
};

use crate::registry_path;

#[derive(Args, Clone)]
pub(crate) struct ProfileArgs {
    #[arg(long)]
    pub(crate) profile: Option<String>,
    #[arg(long)]
    pub(crate) node: Option<String>,
    #[arg(long)]
    pub(crate) registry: Option<String>,
}

pub(crate) fn selected_host_context(
    args: &ProfileArgs,
) -> Result<(HostProfile, Option<HostProfileResolution>), String> {
    let selected = selected_profile(args)?;
    let resolution = selected_resolution(args);
    Ok((selected, resolution))
}

pub(crate) fn selected_profile(args: &ProfileArgs) -> Result<HostProfile, String> {
    if let Some(name) = &args.profile {
        return HostProfile::parse(name);
    }
    let registry = registry_path(args.registry.clone());
    if let Some(node_name) = &args.node {
        return resolve_host_profile(node_name, Some(registry.as_str()));
    }
    match resolve_current_host_profile(Some(registry.as_str())) {
        Ok(profile) => Ok(profile),
        Err(_) => Ok(HostProfile::NetworkNode),
    }
}

pub(crate) fn selected_resolution(args: &ProfileArgs) -> Option<HostProfileResolution> {
    if let Some(name) = &args.profile {
        return Some(HostProfileResolution {
            hostname_observed: env::var("HOSTNAME")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "unknown".to_string()),
            matched_node: None,
            matched_role: None,
            canonical_role: None,
            profile_applied: match HostProfile::parse(name) {
                Ok(profile) => profile.name(),
                Err(_) => "unknown",
            },
            match_source: "explicit_profile",
            host_bootstrap_target_stage: None,
            host_bootstrap_required_domains: Vec::new(),
            registry_path: Some(registry_path(args.registry.clone())),
        });
    }
    let registry = registry_path(args.registry.clone());
    if let Some(node_name) = &args.node {
        return resolve_host_profile_resolution(node_name, Some(registry.as_str())).ok();
    }
    resolve_current_host_profile_resolution(Some(registry.as_str())).ok()
}

pub(crate) fn observed_blocked(observed: &cynic_host::usecases::HostObservationReports) -> bool {
    observed.status != SurfaceStatus::Active
}
