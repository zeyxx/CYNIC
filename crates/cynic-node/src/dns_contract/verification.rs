use super::{
    CANONICAL_DNS_BINARY, CANONICAL_DNS_PROVIDER, DEFAULT_DNS_ZONE, DnsRuntimeContext,
    SERVICE_DNS_COREFILE_FILENAME, SERVICE_DNS_HOSTS_FILENAME, SERVICE_DNS_MANIFEST_FILENAME,
    SERVICE_DNS_README_FILENAME, SERVICE_DNS_RESOLVED_DROPIN_FILENAME,
    SERVICE_DNS_SERVICE_FILENAME, approved_dns_sources, dns_activation_readme_with_context,
    dns_forbidden_patterns, dns_provider_policy_surface_for_zone,
    dns_required_modes, dns_systemd_unit_with_context, forbidden_dns_sources,
    supported_dns_adapters, unadapted_dns_candidates,
};

fn context() -> DnsRuntimeContext {
    DnsRuntimeContext {
        provider: super::DnsProviderKind::CoreDns,
        provider_package: "coredns",
        zone: "labmesh.example".to_string(),
        storage_probe_fqdn: Some("storage.labmesh.example".to_string()),
        package_query_bin: "/portable/bin/dpkg".to_string(),
        rpm_query_bin: "/portable/bin/rpm".to_string(),
        service_manager_kind: "systemd".to_string(),
        service_manager_bin: "/portable/bin/systemctl".to_string(),
        service_enable_bin: "/portable/bin/systemctl".to_string(),
        dns_service_unit: format!("coredns-{}.service", SERVICE_DNS_SERVICE_FILENAME),
        resolved_unit: "systemd-resolved.service".to_string(),
        resolver_command: "/portable/bin/getent".to_string(),
        dns_binary_command: "/portable/bin/coredns".to_string(),
        hosts_path: format!(
            "/portable/etc/coredns/{}",
            cynic_network::dns::SERVICE_DNS_HOSTS_FILENAME
        ),
        corefile_path: "/portable/etc/coredns/Corefile".to_string(),
        service_path: format!(
            "/portable/etc/systemd/system/coredns-{}.service",
            SERVICE_DNS_SERVICE_FILENAME
        ),
        resolved_dropin_path: format!(
            "/portable/etc/systemd/resolved.conf.d/{SERVICE_DNS_RESOLVED_DROPIN_FILENAME}"
        )
        .to_string(),
        bundle_config_filename: SERVICE_DNS_COREFILE_FILENAME,
        bundle_service_filename: SERVICE_DNS_SERVICE_FILENAME,
    }
}

#[kani::proof]
fn proof_dns_bundle_files_shape_is_stable() {
    let files = cynic_network::canonical_dns_bundle_files();

    assert_eq!(files.len(), 6);
    assert_eq!(files[0], SERVICE_DNS_COREFILE_FILENAME);
    assert_eq!(files[1], SERVICE_DNS_README_FILENAME);
    assert_eq!(files[2], SERVICE_DNS_SERVICE_FILENAME);
    assert_eq!(files[3], SERVICE_DNS_RESOLVED_DROPIN_FILENAME);
    assert_eq!(files[4], SERVICE_DNS_MANIFEST_FILENAME);
    assert_eq!(files[5], SERVICE_DNS_HOSTS_FILENAME);
}

#[kani::proof]
fn proof_dns_policy_surface_tracks_requested_zone() {
    let policy = dns_provider_policy_surface_for_zone("labmesh.example");

    assert_eq!(policy.preferred_provider, CANONICAL_DNS_PROVIDER);
    assert_eq!(policy.preferred_binary, CANONICAL_DNS_BINARY);
    assert_eq!(policy.supported_adapters, vec!["coredns", "dnsmasq"]);
    assert_eq!(policy.service_contract, "*.labmesh.example");
}

#[kani::proof]
fn proof_dns_required_modes_are_uniform_0644() {
    let modes = dns_required_modes();

    assert_eq!(modes.hosts, "644");
    assert_eq!(modes.corefile, "644");
    assert_eq!(modes.service, "644");
    assert_eq!(modes.resolved_dropin, "644");
}

#[kani::proof]
fn proof_coredns_unit_uses_injected_binary_and_corefile() {
    let unit = dns_systemd_unit_with_context(&context());

    assert_eq!(
        unit,
        "[Unit]\nDescription=CYNIC Synapse DNS adapter (coredns)\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=simple\nUser=cynic\nGroup=cynic\nExecStart=/portable/bin/coredns -conf /portable/etc/coredns/Corefile\nRestart=on-failure\nRestartSec=5\nNoNewPrivileges=true\nPrivateTmp=true\nProtectHome=true\nProtectSystem=strict\nProtectKernelTunables=true\nProtectKernelModules=true\nProtectControlGroups=true\nRestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX AF_NETLINK\n\n[Install]\nWantedBy=multi-user.target\n"
    );
}

#[kani::proof]
fn proof_dns_activation_readme_uses_explicit_probe_and_service_unit() {
    let readme = dns_activation_readme_with_context(&context());

    assert_eq!(
        readme,
        "# cynic-node DNS activation bundle\n\nThis bundle is non-mutating. Install it only through an approved activation path.\nDo not install these files directly as root without an explicit operator action.\n\nSelected adapter: coredns (coredns)\n\n- Allowed: distribution package or approved internal package mirror.\n- Forbidden: remote installer piped to shell, unverified upstream binary drops.\n- Provider selection requires both a dedicated adapter contract and observed package provenance.\n\nAfter installation, verify:\n\n1. `systemctl is-active coredns-service.service`\n2. `getent hosts storage.labmesh.example`\n3. `cynic network runtime --resolve-dns`\n"
    );
}

#[kani::proof]
fn proof_dns_source_lists_keep_canonical_guardrails() {
    let approved = approved_dns_sources();
    let forbidden = forbidden_dns_sources();
    let supported = supported_dns_adapters();
    let unadapted = unadapted_dns_candidates();
    let patterns = dns_forbidden_patterns();

    assert_eq!(
        approved,
        vec!["distribution_package", "approved_internal_package_mirror",]
    );
    assert_eq!(
        forbidden,
        vec![
            "curl_pipe_shell",
            "wget_pipe_shell",
            "unverified_upstream_binary",
        ]
    );
    assert_eq!(supported, vec!["coredns", "dnsmasq"]);
    assert_eq!(unadapted, vec!["unbound", "bind9", "systemd-resolved"]);
    assert_eq!(patterns[0], "manual /etc/hosts edits as canonical DNS");
}

#[kani::proof]
fn proof_default_dns_zone_constant_is_stable() {
    assert_eq!(DEFAULT_DNS_ZONE, "service.cynic");
}
