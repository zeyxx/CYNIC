use std::fs;
use std::path::Path;

use cynic_host::install_contract::{ROOT_OWNERSHIP, owner_string};
use serde::Serialize;

use crate::adapters::dns_renderer::SynapseDnsRenderer;
use crate::adapters::registry::LocalRegistryAdapter;
use crate::dns_contract::{
    DnsPackagePrecondition, DnsProviderPolicySurface, DnsRequiredModes, DnsTargetPaths,
    SYNAPSE_DNS_COREFILE_FILENAME, SYNAPSE_DNS_HOSTS_FILENAME, SYNAPSE_DNS_MANIFEST_FILENAME,
    SYNAPSE_DNS_README_FILENAME, SYNAPSE_DNS_RESOLVED_DROPIN_FILENAME,
    SYNAPSE_DNS_SERVICE_FILENAME, coredns_systemd_unit_for_zone, dns_activation_checks_for_zone,
    dns_activation_readme_for_zone, dns_bundle_files, dns_forbidden_patterns,
    dns_package_preconditions, dns_provider_policy_surface_for_zone, dns_required_modes,
    dns_target_paths_for_zone,
};
use crate::network_command_surface::NetworkCommandSurface;
use crate::ports::network_interfaces_port::{DnsBundleRendererPort, RegistryTopologyPort};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsBundleReport {
    pub(crate) schema: &'static str,
    mutates: bool,
    target: DnsTargetPaths,
    required_owner: String,
    required_modes: DnsRequiredModes,
    provider_policy: DnsProviderPolicySurface,
    package_preconditions: Vec<DnsPackagePrecondition>,
    activation_checks: Vec<NetworkCommandSurface>,
    forbidden: Vec<&'static str>,
    bundle_dir: String,
    bundle_files: Vec<String>,
}

pub(crate) fn build_dns_bundle(
    registry: &str,
    output_dir: &str,
) -> Result<DnsBundleReport, Box<dyn std::error::Error>> {
    let registry_adapter = LocalRegistryAdapter::new(registry);
    let topology = registry_adapter.fetch_static_topology()?;
    let zone = registry_adapter
        .fetch_network_policy_summary()?
        .synapse_dns_zone
        .unwrap_or_else(crate::dns_contract::default_dns_zone);
    let probe_fqdn = topology
        .iter()
        .flat_map(|record| record.names.iter())
        .find(|name| name.ends_with(&zone))
        .cloned();
    let renderer = SynapseDnsRenderer::new(zone.clone());

    let out = Path::new(output_dir);
    fs::create_dir_all(out)?;
    fs::write(
        out.join(SYNAPSE_DNS_HOSTS_FILENAME),
        renderer.render_hosts(&topology)?,
    )?;
    fs::write(
        out.join(SYNAPSE_DNS_COREFILE_FILENAME),
        renderer.render_corefile()?,
    )?;
    fs::write(
        out.join(SYNAPSE_DNS_SERVICE_FILENAME),
        coredns_systemd_unit_for_zone(&zone),
    )?;
    fs::write(
        out.join(SYNAPSE_DNS_RESOLVED_DROPIN_FILENAME),
        renderer.render_resolved_dropin()?,
    )?;
    fs::write(
        out.join(SYNAPSE_DNS_README_FILENAME),
        dns_activation_readme_for_zone(&zone, probe_fqdn.clone()),
    )?;

    let report = DnsBundleReport {
        schema: "cynic.organ_fleet.dns_activation_bundle.v1",
        mutates: false,
        target: dns_target_paths_for_zone(&zone),
        required_owner: owner_string(ROOT_OWNERSHIP),
        required_modes: dns_required_modes(),
        provider_policy: dns_provider_policy_surface_for_zone(&zone),
        package_preconditions: dns_package_preconditions(),
        activation_checks: dns_activation_checks_for_zone(&zone, probe_fqdn),
        forbidden: dns_forbidden_patterns(),
        bundle_dir: out.to_string_lossy().to_string(),
        bundle_files: dns_bundle_files(),
    };
    fs::write(
        out.join(SYNAPSE_DNS_MANIFEST_FILENAME),
        serde_json::to_string_pretty(&report)? + "\n",
    )?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_host::registry::DEFAULT_REGISTRY_PATH;

    #[test]
    fn bundle_writes_manifest_and_hosts() {
        let out = tempfile::tempdir().expect("tempdir");
        let registry = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(DEFAULT_REGISTRY_PATH);
        let report = build_dns_bundle(
            registry.to_str().expect("registry path"),
            out.path().to_str().expect("path"),
        )
        .expect("bundle");
        assert_eq!(report.schema, "cynic.organ_fleet.dns_activation_bundle.v1");
        assert!(out.path().join(SYNAPSE_DNS_MANIFEST_FILENAME).exists());
        assert!(out.path().join(SYNAPSE_DNS_HOSTS_FILENAME).exists());
    }
}
