use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::adapters::dns_renderer::SynapseDnsRenderer;
use crate::adapters::registry::LocalRegistryAdapter;
use crate::ports::network_interfaces_port::{DnsBundleRendererPort, RegistryTopologyPort};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsBundleReport {
    pub(crate) schema: &'static str,
    mutates: bool,
    target: DnsBundleTarget,
    required_owner: &'static str,
    required_modes: DnsBundleModes,
    provider_policy: ProviderPolicy,
    package_preconditions: Vec<PackagePrecondition>,
    activation_checks: Vec<&'static str>,
    forbidden: Vec<&'static str>,
    bundle_dir: String,
    bundle_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DnsBundleTarget {
    zone: &'static str,
    hosts: &'static str,
    corefile: &'static str,
    service: &'static str,
    resolved_dropin: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct DnsBundleModes {
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

pub(crate) fn build_dns_bundle(
    registry: &str,
    output_dir: &str,
) -> Result<DnsBundleReport, Box<dyn std::error::Error>> {
    let registry_adapter = LocalRegistryAdapter::new(registry);
    let topology = registry_adapter.fetch_static_topology()?;
    let renderer = SynapseDnsRenderer;

    let out = Path::new(output_dir);
    fs::create_dir_all(out)?;
    fs::write(out.join("synapse.hosts"), renderer.render_hosts(&topology)?)?;
    fs::write(out.join("Corefile"), renderer.render_corefile()?)?;
    fs::write(out.join("coredns-synapse.service"), coredns_systemd_unit())?;
    fs::write(out.join("cynic-synapse-dns.conf"), renderer.render_resolved_dropin()?)?;
    fs::write(out.join("README.activation.md"), dns_activation_readme())?;

    let report = DnsBundleReport {
        schema: "cynic.organ_fleet.dns_activation_bundle.v1",
        mutates: false,
        target: DnsBundleTarget {
            zone: "synapse.cynic",
            hosts: "/etc/coredns/synapse.hosts",
            corefile: "/etc/coredns/Corefile",
            service: "/etc/systemd/system/coredns-synapse.service",
            resolved_dropin: "/etc/systemd/resolved.conf.d/cynic-synapse-dns.conf",
        },
        required_owner: "root:root",
        required_modes: DnsBundleModes {
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
        bundle_dir: out.to_string_lossy().to_string(),
        bundle_files: vec![
            "Corefile".to_string(),
            "README.activation.md".to_string(),
            "coredns-synapse.service".to_string(),
            "cynic-synapse-dns.conf".to_string(),
            "manifest.json".to_string(),
            "synapse.hosts".to_string(),
        ],
    };
    fs::write(
        out.join("manifest.json"),
        serde_json::to_string_pretty(&report)? + "\n",
    )?;
    Ok(report)
}

fn coredns_systemd_unit() -> &'static str {
    "[Unit]\nDescription=CYNIC Synapse CoreDNS\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=simple\nExecStart=/usr/bin/env coredns -conf /etc/coredns/Corefile\nRestart=on-failure\nRestartSec=5\n\n[Install]\nWantedBy=multi-user.target\n"
}

fn dns_activation_readme() -> &'static str {
    "# organ-fleet DNS activation bundle\n\nThis bundle is non-mutating. Install it only through an approved activation organ\nor operator path. Cortex sessions must not install these files directly as root.\n\nCoreDNS binary policy:\n\n- Allowed: distribution package or approved internal package mirror.\n- Forbidden: remote installer piped to shell, unverified upstream binary drops.\n- Noncanonical providers such as dnsmasq, unbound or bind9 require their own\n  adapter contract before they can activate Synapse DNS.\n\nAfter installation, verify:\n\n1. `systemctl is-active coredns-synapse.service`\n2. `getent hosts organ-surrealdb.synapse.cynic`\n3. `python3 organs/fleet/src/entrypoints/cli.py mesh status --resolve-dns --include-records`\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_writes_manifest_and_hosts() {
        let out = tempfile::tempdir().expect("tempdir");
        let registry = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("infra/registry.json");
        let report = build_dns_bundle(
            registry.to_str().expect("registry path"),
            out.path().to_str().expect("path"),
        )
        .expect("bundle");
        assert_eq!(report.schema, "cynic.organ_fleet.dns_activation_bundle.v1");
        assert!(out.path().join("manifest.json").exists());
        assert!(out.path().join("synapse.hosts").exists());
    }
}
