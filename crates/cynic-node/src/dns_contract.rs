use std::collections::BTreeMap;

use cynic_host::install_contract::MODE_0644;
use cynic_host::runtime_layout::current_runtime_layout;
use serde::Serialize;

use crate::network_command_surface::{
    NetworkCommandSurface, argv_command, expectation, package_presence_command,
};

#[derive(Debug, Clone)]
struct DnsRuntimeContext {
    zone: String,
    storage_probe_fqdn: Option<String>,
    package_query_bin: String,
    rpm_query_bin: String,
    service_manager_bin: String,
    dns_service_unit: String,
    resolved_unit: String,
    resolver_command: String,
    dns_binary_command: String,
    hosts_path: String,
    corefile_path: String,
    service_path: String,
    resolved_dropin_path: String,
}

pub(crate) const DEFAULT_DNS_ZONE: &str = "synapse.cynic";

pub(crate) const SYNAPSE_DNS_HOSTS_FILENAME: &str = "synapse.hosts";
pub(crate) const SYNAPSE_DNS_COREFILE_FILENAME: &str = "Corefile";
pub(crate) const SYNAPSE_DNS_SERVICE_FILENAME: &str = "coredns-synapse.service";
pub(crate) const SYNAPSE_DNS_RESOLVED_DROPIN_FILENAME: &str = "cynic-synapse-dns.conf";
pub(crate) const SYNAPSE_DNS_MANIFEST_FILENAME: &str = "manifest.json";
pub(crate) const SYNAPSE_DNS_README_FILENAME: &str = "README.activation.md";

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
    pub(crate) canonical_provider: &'static str,
    pub(crate) canonical_binary: &'static str,
    pub(crate) selection_source: &'static str,
    pub(crate) service_contract: String,
    pub(crate) approved_sources: Vec<&'static str>,
    pub(crate) known_noncanonical_candidates: Vec<&'static str>,
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
}

pub(crate) fn approved_dns_sources() -> Vec<&'static str> {
    vec!["distribution_package", "approved_internal_package_mirror"]
}

pub(crate) fn noncanonical_dns_candidates() -> Vec<&'static str> {
    vec!["dnsmasq", "unbound", "bind9", "systemd-resolved"]
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
    vec![
        expectation(
            "package_source_policy",
            "install CoreDNS from a distribution package or approved internal package mirror",
        ),
        package_presence_command(
            "package_presence",
            CANONICAL_DNS_BINARY,
            context.package_query_bin.clone(),
            context.rpm_query_bin.clone(),
        ),
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
        argv_command(
            "resolve_storage_fqdn",
            vec![
                context.resolver_command.clone(),
                "hosts".to_string(),
                context
                    .storage_probe_fqdn
                    .clone()
                    .unwrap_or_else(|| format!("*.{}", context.zone)),
            ],
        ),
    ]
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
        canonical_provider: CANONICAL_DNS_PROVIDER,
        canonical_binary: CANONICAL_DNS_BINARY,
        selection_source: DNS_SELECTION_SOURCE,
        service_contract: format!("*.{zone}"),
        approved_sources: approved_dns_sources(),
        known_noncanonical_candidates: noncanonical_dns_candidates(),
        promotion_rule: DNS_PROMOTION_RULE,
    }
}

pub(crate) fn dns_package_preconditions() -> Vec<DnsPackagePrecondition> {
    vec![DnsPackagePrecondition {
        name: CANONICAL_DNS_PROVIDER,
        binary: CANONICAL_DNS_BINARY,
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

    DnsProviderObservedStatus {
        canonical_provider: CANONICAL_DNS_PROVIDER,
        canonical_binary: CANONICAL_DNS_BINARY,
        canonical_available,
        usable_provider: canonical_available.then_some(CANONICAL_DNS_PROVIDER),
        observed_binaries,
        noncanonical_available,
        blocked_reason: (!canonical_available).then_some("canonical_dns_provider_missing"),
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
    vec![
        SYNAPSE_DNS_COREFILE_FILENAME.to_string(),
        SYNAPSE_DNS_README_FILENAME.to_string(),
        SYNAPSE_DNS_SERVICE_FILENAME.to_string(),
        SYNAPSE_DNS_RESOLVED_DROPIN_FILENAME.to_string(),
        SYNAPSE_DNS_MANIFEST_FILENAME.to_string(),
        SYNAPSE_DNS_HOSTS_FILENAME.to_string(),
    ]
}

pub(crate) fn coredns_systemd_unit_for_zone(zone: &str) -> String {
    coredns_systemd_unit_with_context(&dns_runtime_context_for_zone(zone, None))
}

fn coredns_systemd_unit_with_context(context: &DnsRuntimeContext) -> String {
    format!(
        "[Unit]\nDescription=CYNIC CoreDNS projection for {}\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=simple\nExecStart=/usr/bin/env {} -conf {corefile}\nRestart=on-failure\nRestartSec=5\n\n[Install]\nWantedBy=multi-user.target\n",
        context.zone,
        context.dns_binary_command,
        corefile = context.corefile_path,
    )
}

pub(crate) fn dns_activation_readme_for_zone(zone: &str, probe_fqdn: Option<String>) -> String {
    dns_activation_readme_with_context(&dns_runtime_context_for_zone(zone, probe_fqdn))
}

fn dns_activation_readme_with_context(context: &DnsRuntimeContext) -> String {
    format!(
        "# cynic-node DNS activation bundle\n\nThis bundle is non-mutating. Install it only through an approved activation path.\nDo not install these files directly as root without an explicit operator action.\n\nCoreDNS binary policy:\n\n- Allowed: distribution package or approved internal package mirror.\n- Forbidden: remote installer piped to shell, unverified upstream binary drops.\n- Noncanonical providers such as dnsmasq, unbound or bind9 require their own\n  adapter contract before they can activate this DNS zone.\n\nAfter installation, verify:\n\n1. `systemctl is-active {}`\n2. `getent hosts {}`\n3. `cargo run -q -p cynic-node -- network runtime-status --json --resolve-dns`\n",
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
    DnsRuntimeContext {
        zone: zone.to_string(),
        storage_probe_fqdn,
        package_query_bin: capabilities.package_manager.query.clone(),
        rpm_query_bin: capabilities.package_manager.rpm_query.clone(),
        service_manager_bin: capabilities.service_manager.command.clone(),
        dns_service_unit: capabilities.dns_provider.service_unit.clone(),
        resolved_unit: capabilities.dns_provider.resolved_unit.clone(),
        resolver_command: capabilities.name_service.resolver_command.clone(),
        dns_binary_command: capabilities.dns_provider.command.clone(),
        hosts_path: runtime_layout.dns_hosts_path(),
        corefile_path: runtime_layout.dns_corefile_path(),
        service_path: runtime_layout.dns_service_path(),
        resolved_dropin_path: runtime_layout.dns_resolved_dropin_path(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DnsRuntimeContext, coredns_systemd_unit_with_context, dns_activation_checks_with_context,
        dns_activation_readme_with_context, dns_target_paths_with_context,
    };

    fn context() -> DnsRuntimeContext {
        DnsRuntimeContext {
            zone: "test.zone".to_string(),
            storage_probe_fqdn: Some("storage.test.zone".to_string()),
            package_query_bin: "/usr/bin/dpkg".to_string(),
            rpm_query_bin: "/usr/bin/rpm".to_string(),
            service_manager_bin: "/usr/bin/systemctl".to_string(),
            dns_service_unit: "coredns-synapse.service".to_string(),
            resolved_unit: "systemd-resolved.service".to_string(),
            resolver_command: "/usr/bin/getent".to_string(),
            dns_binary_command: "/usr/bin/coredns".to_string(),
            hosts_path: "/etc/coredns/synapse.hosts".to_string(),
            corefile_path: "/etc/coredns/Corefile".to_string(),
            service_path: "/etc/systemd/system/coredns-synapse.service".to_string(),
            resolved_dropin_path: "/etc/systemd/resolved.conf.d/cynic-synapse-dns.conf".to_string(),
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
        let unit = coredns_systemd_unit_with_context(&context());
        let readme = dns_activation_readme_with_context(&context());
        assert!(unit.contains("/usr/bin/coredns -conf /etc/coredns/Corefile"));
        assert!(readme.contains("coredns-synapse.service"));
        assert!(readme.contains("storage.test.zone"));
    }
}
