mod apply;
mod bundle;
pub(crate) mod execution;
mod policy;
mod types;

pub(crate) use apply::apply_profile;
pub(crate) use bundle::{activation_plan, write_install_bundle};
// Execution records are host substrate proof: node code only adapts and
// schedules them. Keeping this alias preserves node call sites while removing
// a duplicate execution schema from the node layer.
pub(crate) use cynic_host::execution::HostCommandExecution as CommandExecution;

#[cfg(test)]
mod tests {
    use super::{apply_profile, policy::build_policy_document};
    use crate::domain::node_observation::NodeRuntimeCapabilities;

    #[test]
    fn policy_document_only_exposes_profiles_required_by_current_observation() {
        let observation = crate::observe::NodeObservation::synthetic_for_tests(
            NodeRuntimeCapabilities {
                provider_fallback_active: true,
                synapse_route_active: false,
                wireguard_active: false,
                critical_dns_ok: true,
                critical_dns_canonical: true,
                critical_tcp_open: true,
            },
            Some("192.168.0.37".to_string()),
        );
        let policy = build_policy_document(&observation);
        assert!(policy.allowed_profiles.contains_key("synapse-route"));
        assert!(policy.allowed_profiles.contains_key("wireguard-mesh"));
        assert!(
            !policy
                .allowed_profiles
                .contains_key("provider-fallback-route")
        );
    }

    #[test]
    fn apply_requires_explicit_flag_and_allowed_profile() {
        let observation = crate::observe::NodeObservation::synthetic_for_tests(
            NodeRuntimeCapabilities {
                provider_fallback_active: true,
                synapse_route_active: false,
                wireguard_active: false,
                critical_dns_ok: true,
                critical_dns_canonical: true,
                critical_tcp_open: true,
            },
            Some("192.168.0.37".to_string()),
        );
        let report = apply_profile(&observation, "synapse-route", false);
        assert_eq!(report.status, "refused");
        assert!(
            report
                .violations
                .iter()
                .any(|item| item.code == "apply_flag_required")
        );
    }
}
