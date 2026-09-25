<<<<<<< HEAD
use std::collections::BTreeMap;
=======
>>>>>>> github/main
use std::fs;
use std::path::Path;

use cynic_host::install_contract::{ROOT_OWNERSHIP, owner_string};
use serde::Serialize;
<<<<<<< HEAD
use sha2::{Digest, Sha256};
=======
>>>>>>> github/main

use crate::adapters::dns_renderer::SynapseDnsRenderer;
use crate::adapters::registry::LocalRegistryAdapter;
use crate::dns_contract::{
    DnsPackagePrecondition, DnsProviderPolicySurface, DnsRequiredModes, DnsTargetPaths,
<<<<<<< HEAD
    SYNAPSE_DNS_HOSTS_FILENAME, SYNAPSE_DNS_MANIFEST_FILENAME, SYNAPSE_DNS_README_FILENAME,
    SYNAPSE_DNS_RESOLVED_DROPIN_FILENAME, dns_activation_checks_for_zone,
    dns_activation_readme_for_zone, dns_bundle_config_filename, dns_bundle_files,
    dns_bundle_service_filename, dns_forbidden_patterns, dns_package_preconditions,
    dns_provider_config_for_zone, dns_provider_policy_surface_for_zone, dns_required_modes,
    dns_systemd_unit_for_zone, dns_target_paths_for_zone,
=======
    SYNAPSE_DNS_COREFILE_FILENAME, SYNAPSE_DNS_HOSTS_FILENAME, SYNAPSE_DNS_MANIFEST_FILENAME,
    SYNAPSE_DNS_README_FILENAME, SYNAPSE_DNS_RESOLVED_DROPIN_FILENAME,
    SYNAPSE_DNS_SERVICE_FILENAME, coredns_systemd_unit_for_zone, dns_activation_checks_for_zone,
    dns_activation_readme_for_zone, dns_bundle_files, dns_forbidden_patterns,
    dns_package_preconditions, dns_provider_policy_surface_for_zone, dns_required_modes,
    dns_target_paths_for_zone,
>>>>>>> github/main
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
<<<<<<< HEAD
    digests: BTreeMap<String, String>,
=======
>>>>>>> github/main
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
<<<<<<< HEAD
        out.join(dns_bundle_config_filename()),
        dns_provider_config_for_zone(&zone),
    )?;
    fs::write(
        out.join(dns_bundle_service_filename()),
        dns_systemd_unit_for_zone(&zone),
=======
        out.join(SYNAPSE_DNS_COREFILE_FILENAME),
        renderer.render_corefile()?,
    )?;
    fs::write(
        out.join(SYNAPSE_DNS_SERVICE_FILENAME),
        coredns_systemd_unit_for_zone(&zone),
>>>>>>> github/main
    )?;
    fs::write(
        out.join(SYNAPSE_DNS_RESOLVED_DROPIN_FILENAME),
        renderer.render_resolved_dropin()?,
    )?;
    fs::write(
        out.join(SYNAPSE_DNS_README_FILENAME),
        dns_activation_readme_for_zone(&zone, probe_fqdn.clone()),
    )?;

<<<<<<< HEAD
    let bundle_files = dns_bundle_files();
    let digests = bundle_files
        .iter()
        .filter(|name| name.as_str() != SYNAPSE_DNS_MANIFEST_FILENAME)
        .map(|name| {
            let bytes = fs::read(out.join(name))?;
            Ok::<_, std::io::Error>((
                name.clone(),
                format!("sha256:{}", hex::encode(Sha256::digest(bytes))),
            ))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
=======
>>>>>>> github/main
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
<<<<<<< HEAD
        bundle_files,
        digests,
=======
        bundle_files: dns_bundle_files(),
>>>>>>> github/main
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
        assert!(out.path().join(SYNAPSE_DNS_MANIFEST_FILENAME).exists());
        assert!(out.path().join(SYNAPSE_DNS_HOSTS_FILENAME).exists());
<<<<<<< HEAD
        assert_eq!(report.digests.len(), 5);
        assert!(
            report
                .digests
                .values()
                .all(|digest| digest.starts_with("sha256:"))
        );
=======
>>>>>>> github/main
    }
}
