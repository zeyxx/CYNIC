use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use cynic_host::runtime_layout::current_runtime_layout;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::adapters::registry::LocalRegistryAdapter;
use crate::dns_contract::{
    SYNAPSE_DNS_HOSTS_FILENAME, dns_bundle_config_filename, dns_bundle_service_filename,
    dns_target_paths, observe_dns_provider_status,
};
use crate::ports::network_interfaces_port::RegistryTopologyPort;

use super::types::{DnsActivationFileSpec, DnsActivationSpec};

pub(crate) fn resolve_dns_activation_spec(
    registry: &str,
    bundle_dir: &str,
) -> Result<DnsActivationSpec, String> {
    let bundle = Path::new(bundle_dir);
    if !safe_absolute_path(bundle) {
        return Err("dns_activation_bundle_path_unsafe".to_string());
    }
    let manifest_path = bundle.join("manifest.json");
    let manifest: Value = serde_json::from_slice(
        &fs::read(&manifest_path)
            .map_err(|error| format!("dns_activation_manifest_read_failed:{error}"))?,
    )
    .map_err(|error| format!("dns_activation_manifest_invalid:{error}"))?;
    if manifest["schema"].as_str() != Some("cynic.organ_fleet.dns_activation_bundle.v1") {
        return Err("dns_activation_manifest_schema_mismatch".to_string());
    }
    let digests = serde_json::from_value::<BTreeMap<String, String>>(manifest["digests"].clone())
        .map_err(|_| "dns_activation_manifest_digests_missing".to_string())?;
    let targets = dns_target_paths();
    for (field, actual) in [
        ("zone", targets.zone.as_str()),
        ("hosts", targets.hosts.as_str()),
        ("corefile", targets.corefile.as_str()),
        ("service", targets.service.as_str()),
        ("resolved_dropin", targets.resolved_dropin.as_str()),
    ] {
        if manifest
            .pointer(&format!("/target/{field}"))
            .and_then(Value::as_str)
            != Some(actual)
        {
            return Err(format!("dns_activation_manifest_target_mismatch:{field}"));
        }
    }
    let provider = observe_dns_provider_status();
    let provider_name = provider.selected_provider.unwrap_or_default().to_string();
    let provider_package = provider.selected_package.unwrap_or_default().to_string();
    let layout = current_runtime_layout();
    let capabilities = layout.capabilities();
    let (service_unit, config_name, service_name) = if provider_name == "dnsmasq" {
        (
            "cynic-dns-synapse.service".to_string(),
            "dnsmasq.conf".to_string(),
            "cynic-dns-synapse.service".to_string(),
        )
    } else {
        (
            capabilities.dns_provider.service_unit.clone(),
            dns_bundle_config_filename().to_string(),
            dns_bundle_service_filename().to_string(),
        )
    };
    let sources = [
        (
            SYNAPSE_DNS_HOSTS_FILENAME.to_string(),
            targets.hosts.clone(),
        ),
        (config_name, targets.corefile.clone()),
        (service_name, targets.service.clone()),
        (
            "cynic-synapse-dns.conf".to_string(),
            targets.resolved_dropin.clone(),
        ),
    ];
    let mut blockers = Vec::new();
    let mut files = Vec::new();
    for (name, target) in sources {
        let source = bundle.join(&name);
        let Some(expected) = digests.get(&name).cloned() else {
            blockers.push(format!("dns_activation_digest_missing:{name}"));
            continue;
        };
        match digest_regular_file(&source) {
            Ok(observed) if observed == expected => files.push(DnsActivationFileSpec {
                source: source.display().to_string(),
                target,
                sha256: expected,
                mode: 0o644,
            }),
            Ok(_) => blockers.push(format!("dns_activation_bundle_digest_mismatch:{name}")),
            Err(error) => blockers.push(error),
        }
    }
    if provider_name.is_empty() || !provider.package_provenance_verified {
        blockers.push("dns_activation_provider_provenance_unproved".to_string());
    }
    if capabilities.service_manager.kind != "systemd" {
        blockers.push("dns_activation_service_manager_adapter_unsupported".to_string());
    }
    let topology = LocalRegistryAdapter::new(registry)
        .fetch_static_topology()
        .map_err(|error| format!("dns_activation_registry_unavailable:{error}"))?;
    let probe_fqdn = topology
        .iter()
        .flat_map(|record| record.names.iter())
        .find(|name| name.ends_with(&targets.zone))
        .cloned()
        .ok_or_else(|| "dns_activation_probe_fqdn_missing".to_string())?;
    blockers.sort();
    blockers.dedup();
    Ok(DnsActivationSpec {
        schema: "cynic.dns_activation.spec.v1",
        zone: targets.zone,
        bundle_dir: bundle.display().to_string(),
        provider: provider_name,
        provider_package,
        provider_provenance_proved: provider.package_provenance_verified,
        service_unit,
        resolver_unit: capabilities.dns_provider.resolved_unit,
        service_manager_kind: capabilities.service_manager.kind,
        service_manager_bin: capabilities.service_manager.command,
        resolver_bin: capabilities.name_service.resolver_command,
        probe_fqdn,
        files,
        blockers,
    })
}

fn digest_regular_file(path: &Path) -> Result<String, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "dns_activation_bundle_file_unobservable:{}:{error}",
            path.display()
        )
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "dns_activation_bundle_file_unsafe:{}",
            path.display()
        ));
    }
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "dns_activation_bundle_file_read_failed:{}:{error}",
            path.display()
        )
    })?;
    Ok(format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
}

fn safe_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
}
