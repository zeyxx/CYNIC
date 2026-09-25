use crate::host_exec_cli::args::flag_value;
use crate::profiles::HostProfile;
use crate::registry::{
    HostProfileResolution, resolve_current_host_profile, resolve_current_host_profile_resolution,
    resolve_host_profile, resolve_host_profile_resolution,
};

pub(crate) fn selected_profile(args: &[String]) -> Result<HostProfile, String> {
    if let Some(name) = flag_value(args, "--profile") {
        return HostProfile::parse(&name);
    }
    if let Some(node_name) = flag_value(args, "--node") {
        let registry_path = flag_value(args, "--registry");
        return resolve_host_profile(&node_name, registry_path.as_deref());
    }
    let registry_path = flag_value(args, "--registry");
<<<<<<< HEAD
    resolve_current_host_profile(registry_path.as_deref())
=======
    match resolve_current_host_profile(registry_path.as_deref()) {
        Ok(profile) => Ok(profile),
        Err(_) => Ok(HostProfile::NetworkNode),
    }
>>>>>>> github/main
}

pub(crate) fn selected_resolution(args: &[String]) -> Option<HostProfileResolution> {
    if let Some(name) = flag_value(args, "--profile") {
        return Some(HostProfileResolution {
            hostname_observed: std::env::var("HOSTNAME")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "unknown".to_string()),
            matched_node: None,
            matched_role: None,
            canonical_role: None,
            profile_applied: match HostProfile::parse(&name) {
                Ok(profile) => profile.name(),
                Err(_) => "unknown",
            },
            match_source: "explicit_profile",
            host_bootstrap_target_stage: None,
            host_bootstrap_required_domains: Vec::new(),
            registry_path: flag_value(args, "--registry"),
        });
    }
    if let Some(node_name) = flag_value(args, "--node") {
        let registry_path = flag_value(args, "--registry");
        return resolve_host_profile_resolution(&node_name, registry_path.as_deref()).ok();
    }
    let registry_path = flag_value(args, "--registry");
    resolve_current_host_profile_resolution(registry_path.as_deref()).ok()
}
