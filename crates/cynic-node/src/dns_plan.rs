use cynic_host::install_contract::{ROOT_OWNERSHIP, owner_string};
use serde::Serialize;

use crate::dns_contract::{
    DnsPackagePrecondition, DnsProviderPolicySurface, DnsRequiredModes, DnsTargetPaths,
    dns_activation_checks, dns_forbidden_patterns, dns_package_preconditions,
    dns_provider_policy_surface, dns_required_modes, dns_target_paths,
};
use crate::network_command_surface::NetworkCommandSurface;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsPlanReport {
    pub(crate) schema: &'static str,
    mutates: bool,
    target: DnsTargetPaths,
    required_owner: String,
    required_modes: DnsRequiredModes,
    provider_policy: DnsProviderPolicySurface,
    package_preconditions: Vec<DnsPackagePrecondition>,
    activation_checks: Vec<NetworkCommandSurface>,
    forbidden: Vec<&'static str>,
}

pub(crate) fn build_dns_plan() -> DnsPlanReport {
    DnsPlanReport {
        schema: "cynic.organ_fleet.dns_activation_plan.v1",
        mutates: false,
        target: dns_target_paths(),
        required_owner: owner_string(ROOT_OWNERSHIP),
        required_modes: dns_required_modes(),
        provider_policy: dns_provider_policy_surface(),
        package_preconditions: dns_package_preconditions(),
        activation_checks: dns_activation_checks(),
        forbidden: dns_forbidden_patterns(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_shape_is_stable() {
        let plan = build_dns_plan();
        assert_eq!(plan.schema, "cynic.organ_fleet.dns_activation_plan.v1");
    }
}
