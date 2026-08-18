use std::collections::BTreeMap;

use cynic_host::install_contract::MODE_0644;
use cynic_host::runtime_layout::current_runtime_layout;
use serde::Serialize;

use crate::network_command_surface::{
    NetworkCommandSurface, argv_command, expectation, package_presence_command,
};

#[cfg(kani)]
mod verification;

#[derive(Debug, Clone)]
struct DnsRuntimeContext {
    provider: DnsProviderKind,
    provider_package: &'static str,
    zone: String,
    storage_probe_fqdn: Option<String>,
    package_query_bin: String,
    rpm_query_bin: String,
    service_manager_kind: String,
    service_manager_bin: String,
    service_enable_bin: String,
    dns_service_unit: String,
    resolved_unit: String,
    resolver_command: String,
    dns_binary_command: String,
    hosts_path: String,
    corefile_path: String,
    service_path: String,
    resolved_dropin_path: String,
    bundle_config_filename: &'static str,
    bundle_service_filename: &'static str,
}

pub(crate) const DEFAULT_DNS_ZONE: &str = "synapse.cynic";

pub(crate) const SYNAPSE_DNS_HOSTS_FILENAME: &str = "synapse.hosts";
pub(crate) const SYNAPSE_DNS_COREFILE_FILENAME: &str = "Corefile";
pub(crate) const SYNAPSE_DNS_SERVICE_FILENAME: &str = "coredns-synapse.service";
pub(crate) const SYNAPSE_DNS_RESOLVED_DROPIN_FILENAME: &str = "cynic-synapse-dns.conf";
pub(crate) const SYNAPSE_DNS_MANIFEST_FILENAME: &str = "manifest.json";
pub(crate) const SYNAPSE_DNS_README_FILENAME: &str = "README.activation.md";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DnsProviderKind {
    CoreDns,
    Dnsmasq,
}

impl DnsProviderKind {
    fn name(self) -> &'static str {
        match self {
            Self::CoreDns => "coredns",
            Self::Dnsmasq => "dnsmasq",
        }
    }

    fn binary(self) -> &'static str {
        self.name()
    }

    fn package(self) -> &'static str {
        match self {
            Self::CoreDns => "coredns",
            Self::Dnsmasq => "dnsmasq-base",
        }
    }
}

pub(crate) const CANONICAL_DNS_PROVIDER: &str = "coredns";
pub(crate) const CANONICAL_DNS_BINARY: &str = "coredns";
pub(crate) const DNS_SELECTION_SOURCE: &str = "cynic-node Synapse DNS activation contract";
pub(crate) const DNS_PROMOTION_RULE: &str = "A candidate provider becomes usable only after a dedicated adapter contract, bundle renderer, service template, and activation-status checks are added.";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsTargetPaths {
    pub(crate) zone: String,
    pub(crate) hosts: String,
    pub(crate) corefile: String,
    pub(crate) service: String,
    pub(crate) resolved_dropin: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsRequiredModes {
    pub(crate) hosts: String,
    pub(crate) corefile: String,
    pub(crate) service: String,
    pub(crate) resolved_dropin: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsProviderPolicySurface {
    pub(crate) preferred_provider: &'static str,
    pub(crate) preferred_binary: &'static str,
    pub(crate) supported_adapters: Vec<&'static str>,
    pub(crate) selection_source: &'static str,
    pub(crate) service_contract: String,
    pub(crate) approved_sources: Vec<&'static str>,
    pub(crate) known_unadapted_candidates: Vec<&'static str>,
    pub(crate) promotion_rule: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsPackagePrecondition {
    pub(crate) name: &'static str,
    pub(crate) binary: &'static str,
    pub(crate) allowed_sources: Vec<&'static str>,
    pub(crate) forbidden_sources: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsProviderObservedStatus {
    pub(crate) canonical_provider: &'static str,
    pub(crate) canonical_binary: &'static str,
    pub(crate) canonical_available: bool,
    pub(crate) usable_provider: Option<&'static str>,
    pub(crate) observed_binaries: BTreeMap<&'static str, Option<String>>,
    pub(crate) noncanonical_available: Vec<&'static str>,
    pub(crate) blocked_reason: Option<&'static str>,
    pub(crate) selected_provider: Option<&'static str>,
    pub(crate) selected_package: Option<&'static str>,
    pub(crate) package_provenance_verified: bool,
}

pub(crate) fn approved_dns_sources() -> Vec<&'static str> {
    vec!["distribution_package", "approved_internal_package_mirror"]
}

pub(crate) fn supported_dns_adapters() -> Vec<&'static str> {
    vec!["coredns", "dnsmasq"]
}

pub(crate) fn unadapted_dns_candidates() -> Vec<&'static str> {
    vec!["unbound", "bind9", "systemd-resolved"]
}

pub(crate) fn forbidden_dns_sources() -> Vec<&'static str> {
    vec![
        "curl_pipe_shell",
        "wget_pipe_shell",
        "unverified_upstream_binary",
    ]
}

pub(crate) fn dns_activation_checks() -> Vec<NetworkCommandSurface> {
    dns_activation_checks_with_context(&dns_runtime_context())
}

pub(crate) fn dns_activation_checks_for_zone(
    zone: &str,
    probe_fqdn: Option<String>,
) -> Vec<NetworkCommandSurface> {
    dns_activation_checks_with_context(&dns_runtime_context_for_zone(zone, probe_fqdn))
}

fn dns_activation_checks_with_context(context: &DnsRuntimeContext) -> Vec<NetworkCommandSurface> {
    let mut checks = vec![
        expectation(
            "package_source_policy",
            format!(
                "use {} only from a distribution package or approved internal package mirror",
                context.provider_package
            ),
        ),
        package_presence_command(
            "package_presence",
            context.provider_package,
            context.package_query_bin.clone(),
            context.rpm_query_bin.clone(),
        ),
    ];

    checks.extend(service_manager_activation_checks(context));
    checks.push(argv_command(
        "resolve_storage_fqdn",
        vec![
            context.resolver_command.clone(),
            "hosts".to_string(),
            context
                .storage_probe_fqdn
                .clone()
                .unwrap_or_else(|| format!("*.{}", context.zone)),
        ],
    ));
    checks
}

fn service_manager_activation_checks(context: &DnsRuntimeContext) -> Vec<NetworkCommandSurface> {
    match context.service_manager_kind.as_str() {
        "openrc" => vec![
            argv_command(
                "enable_dns_service",
                vec![
                    context.service_enable_bin.clone(),
                    "add".to_string(),
                    openrc_service_name(&context.dns_service_unit),
                    "default".to_string(),
                ],
            ),
            argv_command(
                "start_dns_service",
                vec![
                    context.service_manager_bin.clone(),
                    openrc_service_name(&context.dns_service_unit),
                    "start".to_string(),
                ],
            ),
            expectation(
                "resolver_integration",
                "OpenRC resolver integration requires a dedicated adapter; do not emit resolver-daemon commands from another service-manager family",
            ),
        ],
        _ => vec![
            argv_command(
                "daemon_reload",
                vec![
                    context.service_manager_bin.clone(),
                    "daemon-reload".to_string(),
                ],
            ),
            argv_command(
                "enable_dns_service",
                vec![
                    context.service_manager_bin.clone(),
                    "enable".to_string(),
                    "--now".to_string(),
                    context.dns_service_unit.clone(),
                ],
            ),
            argv_command(
                "restart_resolved",
                vec![
                    context.service_manager_bin.clone(),
                    "restart".to_string(),
                    context.resolved_unit.clone(),
                ],
            ),
        ],
    }
}

fn openrc_service_name(unit: &str) -> String {
    unit.strip_suffix(".service").unwrap_or(unit).to_string()
}

pub(crate) fn dns_target_paths() -> DnsTargetPaths {
    dns_target_paths_with_context(&dns_runtime_context())
}

pub(crate) fn dns_target_paths_for_zone(zone: &str) -> DnsTargetPaths {
    dns_target_paths_with_context(&dns_runtime_context_for_zone(zone, None))
}

fn dns_target_paths_with_context(context: &DnsRuntimeContext) -> DnsTargetPaths {
    DnsTargetPaths {
        zone: context.zone.clone(),
        hosts: context.hosts_path.clone(),
        corefile: context.corefile_path.clone(),
        service: context.service_path.clone(),
        resolved_dropin: context.resolved_dropin_path.clone(),
    }
}

pub(crate) fn dns_required_modes() -> DnsRequiredModes {
    DnsRequiredModes {
        hosts: MODE_0644.octal.to_string(),
        corefile: MODE_0644.octal.to_string(),
        service: MODE_0644.octal.to_string(),
        resolved_dropin: MODE_0644.octal.to_string(),
    }
}

pub(crate) fn dns_provider_policy_surface() -> DnsProviderPolicySurface {
    dns_provider_policy_surface_for_zone(&dns_runtime_context().zone)
}

pub(crate) fn dns_provider_policy_surface_for_zone(zone: &str) -> DnsProviderPolicySurface {
    DnsProviderPolicySurface {
        preferred_provider: CANONICAL_DNS_PROVIDER,
        preferred_binary: CANONICAL_DNS_BINARY,
        supported_adapters: supported_dns_adapters(),
        selection_source: DNS_SELECTION_SOURCE,
        service_contract: format!("*.{zone}"),
        approved_sources: approved_dns_sources(),
        known_unadapted_candidates: unadapted_dns_candidates(),
        promotion_rule: DNS_PROMOTION_RULE,
    }
}

pub(crate) fn dns_package_preconditions() -> Vec<DnsPackagePrecondition> {
    let context = dns_runtime_context();
    vec![DnsPackagePrecondition {
        name: context.provider_package,
        binary: context.provider.binary(),
        allowed_sources: approved_dns_sources(),
        forbidden_sources: forbidden_dns_sources(),
    }]
}

pub(crate) fn observe_dns_provider_status() -> DnsProviderObservedStatus {
    let mut observed_binaries = BTreeMap::new();
    let mut noncanonical_available = Vec::new();
    for binary in ["coredns", "dnsmasq", "unbound", "named", "resolvectl"] {
        let path = find_on_path(binary);
        if binary != CANONICAL_DNS_BINARY && path.is_some() {
            noncanonical_available.push(binary);
        }
        observed_binaries.insert(binary, path);
    }
    let canonical_available = observed_binaries
        .get(CANONICAL_DNS_BINARY)
        .and_then(|value| value.as_ref())
        .is_some();
    let coredns_provenance = observed_binaries
        .get(CANONICAL_DNS_BINARY)
        .and_then(|value| value.as_deref())
        .is_some_and(|path| package_owns_binary("coredns", path));
    let dnsmasq_path = observed_binaries
        .get("dnsmasq")
        .and_then(|value| value.as_deref());
    let dnsmasq_provenance =
        dnsmasq_path.is_some_and(|path| package_owns_binary("dnsmasq-base", path));
    let selected = if coredns_provenance {
        Some(DnsProviderKind::CoreDns)
    } else if dnsmasq_provenance {
        Some(DnsProviderKind::Dnsmasq)
    } else {
        None
    };

    DnsProviderObservedStatus {
        canonical_provider: CANONICAL_DNS_PROVIDER,
        canonical_binary: CANONICAL_DNS_BINARY,
        canonical_available,
        usable_provider: selected.map(DnsProviderKind::name),
        observed_binaries,
        noncanonical_available,
        blocked_reason: selected
            .is_none()
            .then_some("supported_dns_provider_missing"),
        selected_provider: selected.map(DnsProviderKind::name),
        selected_package: selected.map(DnsProviderKind::package),
        package_provenance_verified: selected.is_some(),
    }
}

fn package_owns_binary(package: &str, binary: &str) -> bool {
    let layout = current_runtime_layout();
    let package_manager = layout.capabilities().package_manager;
    match package_manager.kind {
        cynic_host::runtime_layout::PackageManagerKind::Apt => {
            cynic_host::process_observation::command_stdout(&package_manager.query, &["-S", binary])
                .is_some_and(|output| {
                    output
                        .lines()
                        .any(|line| line.starts_with(&format!("{package}:")))
                })
        }
        cynic_host::runtime_layout::PackageManagerKind::Dnf
        | cynic_host::runtime_layout::PackageManagerKind::Rpm => {
            cynic_host::process_observation::command_stdout(
                &package_manager.rpm_query,
                &["-qf", binary],
            )
            .is_some_and(|output| output.trim().starts_with(package))
        }
        _ => false,
    }
}

fn find_on_path(binary: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for entry in std::env::split_paths(&path) {
        let candidate = entry.join(binary);
        if candidate.is_file() {
            return Some(candidate.display().to_string());
        }
    }
    None
}

pub(crate) fn synapse_dns_hosts_path() -> String {
    dns_runtime_context().hosts_path
}

pub(crate) fn synapse_dns_corefile_path() -> String {
    dns_runtime_context().corefile_path
}

pub(crate) fn synapse_dns_resolved_dropin_path() -> String {
    dns_runtime_context().resolved_dropin_path
}

pub(crate) fn default_dns_zone() -> String {
    dns_runtime_context().zone
}

pub(crate) fn dns_forbidden_patterns() -> Vec<&'static str> {
    vec![
        "manual /etc/hosts edits as canonical DNS",
        "192.168.* service endpoint",
        "remote installer piped to shell",
        "wget installer piped to shell",
        "unverified upstream CoreDNS binary",
        "automatic fallback to a noncanonical DNS provider without an adapter contract",
    ]
}

pub(crate) fn dns_bundle_files() -> Vec<String> {
    let context = dns_runtime_context();
    vec![
        context.bundle_config_filename.to_string(),
        SYNAPSE_DNS_README_FILENAME.to_string(),
        context.bundle_service_filename.to_string(),
        SYNAPSE_DNS_RESOLVED_DROPIN_FILENAME.to_string(),
        SYNAPSE_DNS_MANIFEST_FILENAME.to_string(),
        SYNAPSE_DNS_HOSTS_FILENAME.to_string(),
    ]
}

pub(crate) fn dns_bundle_config_filename() -> &'static str {
    dns_runtime_context().bundle_config_filename
}

pub(crate) fn dns_bundle_service_filename() -> &'static str {
    dns_runtime_context().bundle_service_filename
}

pub(crate) fn dns_provider_config_for_zone(zone: &str) -> String {
    let context = dns_runtime_context_for_zone(zone, None);
    match context.provider {
        DnsProviderKind::CoreDns => format!(
            "{zone}:1053 {{\n    bind 127.0.0.1\n    errors\n    hosts {} {{\n        fallthrough\n    }}\n}}\n",
            context.hosts_path,
        ),
        DnsProviderKind::Dnsmasq => format!(
            "# Generated by CYNIC for {zone}.\nno-resolv\nno-hosts\ndomain-needed\nbogus-priv\nbind-interfaces\nlisten-address=127.0.0.1\nport=1053\naddn-hosts={}\ncache-size=0\n",
            context.hosts_path
        ),
    }
}

pub(crate) fn dns_systemd_unit_for_zone(zone: &str) -> String {
    dns_systemd_unit_with_context(&dns_runtime_context_for_zone(zone, None))
}

fn dns_systemd_unit_with_context(context: &DnsRuntimeContext) -> String {
    let exec_start = match context.provider {
        DnsProviderKind::CoreDns => format!(
            "{} -conf {}",
            context.dns_binary_command, context.corefile_path
        ),
        DnsProviderKind::Dnsmasq => format!(
            "{} --keep-in-foreground --conf-file={} --pid-file=",
            context.dns_binary_command, context.corefile_path
        ),
    };
    format!(
        "[Unit]\nDescription=CYNIC Synapse DNS adapter ({provider})\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=simple\nUser=cynic\nGroup=cynic\nExecStart={exec_start}\nRestart=on-failure\nRestartSec=5\nNoNewPrivileges=true\nPrivateTmp=true\nProtectHome=true\nProtectSystem=strict\nProtectKernelTunables=true\nProtectKernelModules=true\nProtectControlGroups=true\nRestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX AF_NETLINK\n\n[Install]\nWantedBy=multi-user.target\n",
        provider = context.provider.name(),
    )
}

pub(crate) fn dns_activation_readme_for_zone(zone: &str, probe_fqdn: Option<String>) -> String {
    dns_activation_readme_with_context(&dns_runtime_context_for_zone(zone, probe_fqdn))
}

fn dns_activation_readme_with_context(context: &DnsRuntimeContext) -> String {
    format!(
        "# cynic-node DNS activation bundle\n\nThis bundle is non-mutating. Install it only through an approved activation path.\nDo not install these files directly as root without an explicit operator action.\n\nSelected adapter: {} ({})\n\n- Allowed: distribution package or approved internal package mirror.\n- Forbidden: remote installer piped to shell, unverified upstream binary drops.\n- Provider selection requires both a dedicated adapter contract and observed package provenance.\n\nAfter installation, verify:\n\n1. `systemctl is-active {}`\n2. `getent hosts {}`\n3. `cynic network runtime --resolve-dns`\n",
        context.provider.name(),
        context.provider_package,
        context.dns_service_unit,
        context
            .storage_probe_fqdn
            .as_deref()
            .unwrap_or("<registry-service-fqdn>")
    )
}

fn dns_runtime_context() -> DnsRuntimeContext {
    dns_runtime_context_for_zone(
        &std::env::var("CYNIC_DNS_ZONE")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_DNS_ZONE.to_string()),
        std::env::var("CYNIC_DNS_PROBE_FQDN")
            .ok()
            .filter(|value| !value.trim().is_empty()),
    )
}

fn dns_runtime_context_for_zone(
    zone: &str,
    storage_probe_fqdn: Option<String>,
) -> DnsRuntimeContext {
    let runtime_layout = current_runtime_layout();
    let capabilities = runtime_layout.capabilities();
    let observed = observe_dns_provider_status();
    let provider = match observed.selected_provider {
        Some("dnsmasq") => DnsProviderKind::Dnsmasq,
        _ => DnsProviderKind::CoreDns,
    };
    let (
        dns_service_unit,
        dns_binary_command,
        hosts_path,
        corefile_path,
        service_path,
        bundle_config_filename,
        bundle_service_filename,
    ) = match provider {
        DnsProviderKind::CoreDns => (
            capabilities.dns_provider.service_unit.clone(),
            capabilities.dns_provider.command.clone(),
            runtime_layout.dns_hosts_path(),
            runtime_layout.dns_corefile_path(),
            runtime_layout.dns_service_path(),
            SYNAPSE_DNS_COREFILE_FILENAME,
            SYNAPSE_DNS_SERVICE_FILENAME,
        ),
        DnsProviderKind::Dnsmasq => (
            "cynic-dns-synapse.service".to_string(),
            observed
                .observed_binaries
                .get("dnsmasq")
                .and_then(Clone::clone)
                .unwrap_or_else(|| "/usr/sbin/dnsmasq".to_string()),
            "/etc/cynic/dns/synapse.hosts".to_string(),
            "/etc/cynic/dns/dnsmasq.conf".to_string(),
            "/etc/systemd/system/cynic-dns-synapse.service".to_string(),
            "dnsmasq.conf",
            "cynic-dns-synapse.service",
        ),
    };
    DnsRuntimeContext {
        provider,
        provider_package: provider.package(),
        zone: zone.to_string(),
        storage_probe_fqdn,
        package_query_bin: capabilities.package_manager.query.clone(),
        rpm_query_bin: capabilities.package_manager.rpm_query.clone(),
        service_manager_kind: capabilities.service_manager.kind.clone(),
        service_manager_bin: capabilities.service_manager.command.clone(),
        service_enable_bin: runtime_layout.rc_update_bin(),
        dns_service_unit,
        resolved_unit: capabilities.dns_provider.resolved_unit.clone(),
        resolver_command: capabilities.name_service.resolver_command.clone(),
        dns_binary_command,
        hosts_path,
        corefile_path,
        service_path,
        resolved_dropin_path: runtime_layout.dns_resolved_dropin_path(),
        bundle_config_filename,
        bundle_service_filename,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DnsProviderKind, DnsRuntimeContext, dns_activation_checks_with_context,
        dns_activation_readme_with_context, dns_provider_config_for_zone,
        dns_systemd_unit_with_context, dns_target_paths_with_context,
    };

    fn context() -> DnsRuntimeContext {
        DnsRuntimeContext {
            provider: DnsProviderKind::CoreDns,
            provider_package: "coredns",
            zone: "test.zone".to_string(),
            storage_probe_fqdn: Some("storage.test.zone".to_string()),
            package_query_bin: "/usr/bin/dpkg".to_string(),
            rpm_query_bin: "/usr/bin/rpm".to_string(),
            service_manager_kind: "systemd".to_string(),
            service_manager_bin: "/usr/bin/systemctl".to_string(),
            service_enable_bin: "/usr/bin/systemctl".to_string(),
            dns_service_unit: "coredns-synapse.service".to_string(),
            resolved_unit: "systemd-resolved.service".to_string(),
            resolver_command: "/usr/bin/getent".to_string(),
            dns_binary_command: "/usr/bin/coredns".to_string(),
            hosts_path: "/etc/coredns/synapse.hosts".to_string(),
            corefile_path: "/etc/coredns/Corefile".to_string(),
            service_path: "/etc/systemd/system/coredns-synapse.service".to_string(),
            resolved_dropin_path: "/etc/systemd/resolved.conf.d/cynic-synapse-dns.conf".to_string(),
            bundle_config_filename: "Corefile",
            bundle_service_filename: "coredns-synapse.service",
        }
    }

    #[test]
    fn dns_paths_and_commands_use_injected_context() {
        let paths = dns_target_paths_with_context(&context());
        let checks = dns_activation_checks_with_context(&context());
        assert_eq!(paths.zone, "test.zone");
        assert_eq!(paths.corefile, "/etc/coredns/Corefile");
        assert!(
            crate::network_command_surface::render_command(&checks[1]).contains("/usr/bin/dpkg")
        );
        assert!(
            crate::network_command_surface::render_command(&checks[2])
                .contains("/usr/bin/systemctl daemon-reload")
        );
        assert!(
            crate::network_command_surface::render_command(&checks[5])
                .contains("storage.test.zone")
        );
    }

    #[test]
    fn dns_unit_and_readme_use_injected_context() {
        let unit = dns_systemd_unit_with_context(&context());
        let readme = dns_activation_readme_with_context(&context());
        assert!(unit.contains("/usr/bin/coredns -conf /etc/coredns/Corefile"));
        assert!(readme.contains("coredns-synapse.service"));
        assert!(readme.contains("storage.test.zone"));
    }

    #[test]
    fn coredns_is_private_to_the_declared_zone_and_loopback() {
        let corefile = dns_provider_config_for_zone("test.zone");

        assert!(corefile.starts_with("test.zone:1053 {"));
        assert!(corefile.contains("bind 127.0.0.1"));
        assert!(!corefile.starts_with(".:1053 {"));
    }

    #[test]
    fn openrc_activation_checks_do_not_emit_systemd_commands() {
        let mut context = context();
        context.service_manager_kind = "openrc".to_string();
        context.service_manager_bin = "/sbin/rc-service".to_string();
        context.service_enable_bin = "/sbin/rc-update".to_string();

        let rendered = dns_activation_checks_with_context(&context)
            .iter()
            .map(crate::network_command_surface::render_command)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("/sbin/rc-update add coredns-synapse default"));
        assert!(rendered.contains("/sbin/rc-service coredns-synapse start"));
        assert!(rendered.contains("OpenRC resolver integration requires a dedicated adapter"));
        assert!(!rendered.contains("daemon-reload"));
        assert!(!rendered.contains("enable --now"));
        assert!(!rendered.contains("systemd-resolved"));
    }
}
