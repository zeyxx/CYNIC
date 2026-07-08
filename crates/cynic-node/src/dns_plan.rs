use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsPlanReport {
    pub(crate) schema: &'static str,
    mutates: bool,
    target: DnsPlanTarget,
    required_owner: &'static str,
    required_modes: DnsPlanModes,
    provider_policy: ProviderPolicy,
    package_preconditions: Vec<PackagePrecondition>,
    activation_checks: Vec<&'static str>,
    forbidden: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct DnsPlanTarget {
    zone: &'static str,
    hosts: &'static str,
    corefile: &'static str,
    service: &'static str,
    resolved_dropin: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct DnsPlanModes {
    hosts: &'static str,
    corefile: &'static str,
    service: &'static str,
    resolved_dropin: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderPolicy {
    canonical_provider: &'static str,
    canonical_binary: &'static str,
    selection_source: &'static str,
    service_contract: &'static str,
    approved_sources: Vec<&'static str>,
    known_noncanonical_candidates: Vec<&'static str>,
    promotion_rule: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct PackagePrecondition {
    name: &'static str,
    binary: &'static str,
    allowed_sources: Vec<&'static str>,
    forbidden_sources: Vec<&'static str>,
}

pub(crate) fn build_dns_plan() -> DnsPlanReport {
    DnsPlanReport {
        schema: "cynic.organ_fleet.dns_activation_plan.v1",
        mutates: false,
        target: DnsPlanTarget {
            zone: "synapse.cynic",
            hosts: "/etc/coredns/synapse.hosts",
            corefile: "/etc/coredns/Corefile",
            service: "/etc/systemd/system/coredns-synapse.service",
            resolved_dropin: "/etc/systemd/resolved.conf.d/cynic-synapse-dns.conf",
        },
        required_owner: "root:root",
        required_modes: DnsPlanModes {
            hosts: "0644",
            corefile: "0644",
            service: "0644",
            resolved_dropin: "0644",
        },
        provider_policy: ProviderPolicy {
            canonical_provider: "coredns",
            canonical_binary: "coredns",
            selection_source: "organ-fleet Synapse DNS activation contract",
            service_contract: "*.synapse.cynic",
            approved_sources: vec!["distribution_package", "approved_internal_package_mirror"],
            known_noncanonical_candidates: vec!["dnsmasq", "unbound", "bind9", "systemd-resolved"],
            promotion_rule: "A candidate provider becomes usable only after a dedicated adapter contract, bundle renderer, service template, and activation-status checks are added.",
        },
        package_preconditions: vec![PackagePrecondition {
            name: "coredns",
            binary: "coredns",
            allowed_sources: vec!["distribution_package", "approved_internal_package_mirror"],
            forbidden_sources: vec![
                "curl_pipe_shell",
                "wget_pipe_shell",
                "unverified_upstream_binary",
            ],
        }],
        activation_checks: vec![
            "install CoreDNS from a distribution package or approved internal package mirror",
            "command -v coredns",
            "install -o root -g root -m 0644 synapse.hosts /etc/coredns/synapse.hosts",
            "install -o root -g root -m 0644 Corefile /etc/coredns/Corefile",
            "install -o root -g root -m 0644 coredns-synapse.service /etc/systemd/system/coredns-synapse.service",
            "install -o root -g root -m 0644 cynic-synapse-dns.conf /etc/systemd/resolved.conf.d/cynic-synapse-dns.conf",
            "systemctl daemon-reload",
            "systemctl enable --now coredns-synapse.service",
            "systemctl restart systemd-resolved.service",
            "getent hosts organ-surrealdb.synapse.cynic resolves 10.20.0.3",
        ],
        forbidden: vec![
            "manual /etc/hosts edits as canonical DNS",
            "192.168.* service endpoint",
            "remote installer piped to shell",
            "wget installer piped to shell",
            "unverified upstream CoreDNS binary",
            "automatic fallback to a noncanonical DNS provider without an adapter contract",
        ],
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
