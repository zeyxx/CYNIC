use std::fs;
use std::path::PathBuf;

use super::*;

fn write_registry(root: &std::path::Path) -> PathBuf {
    let path = root.join("registry.json");
    fs::write(
        &path,
        r#"{
          "network_policy": {
            "canonical_service_plane": "synapse",
            "planes": {
              "synapse": { "cidr": "10.20.0.0/24", "dns_zone": "synapse.cynic", "canonical": true },
              "proxmox_guest": { "cidr": "10.10.10.0/24", "canonical": false }
            },
            "reservations": [
              { "name": "host-proxmox", "synapse": "10.20.0.1", "dns": ["pve.synapse.cynic"] },
              { "name": "builder", "synapse": "10.20.0.5", "dns": ["builder.synapse.cynic"] },
              { "name": "kernel", "synapse": "10.20.0.3", "dns": ["cynic-kernel-primary.synapse.cynic"] }
            ]
          },
          "nodes": [
            {
              "name": "host-proxmox",
              "role": "hypervisor",
              "network": { "local": "10.10.10.1", "synapse": "10.20.0.1" }
            },
            {
              "name": "builder",
              "role": "build_runner",
              "status": "active",
              "aliases": ["builder-ssh"],
              "network": { "local": "10.10.10.5", "synapse": "10.20.0.5" },
              "services": [
                { "name": "ssh", "port": 22 }
              ],
              "vms": [
                { "name": "builder", "aliases": ["build-runner"] }
              ]
            },
            {
              "name": "kernel",
              "role": "kernel_primary",
              "status": "active",
              "network": { "local": "10.10.10.3", "synapse": "10.20.0.3" },
              "services": [
                { "name": "cynic-kernel", "port": 3030 }
              ]
            }
          ]
        }"#,
    )
    .expect("write registry");
    path
}

#[test]
fn fetches_static_topology_with_aliases() {
    let dir = tempfile::tempdir().expect("tempdir");
    let adapter = LocalRegistryAdapter::new(write_registry(dir.path()));

    let topology = adapter.fetch_static_topology().expect("topology");

    assert!(
        topology
            .iter()
            .any(|item| item.node == "builder" && item.ip == "10.20.0.5")
    );
    assert!(
        topology
            .iter()
            .any(|item| item.node == "builder-ssh" && item.ip == "10.20.0.5")
    );
    assert!(
        topology
            .iter()
            .any(|item| item.node == "build-runner" && item.ip == "10.20.0.5")
    );
}

#[test]
fn fetches_network_policy_summary_and_route() {
    let dir = tempfile::tempdir().expect("tempdir");
    let adapter = LocalRegistryAdapter::new(write_registry(dir.path()));

    let summary = adapter.fetch_network_policy_summary().expect("summary");
    let route = adapter.fetch_provider_fallback_route().expect("route");

    assert_eq!(summary.canonical_service_plane, "synapse");
    assert_eq!(summary.synapse_cidr.as_deref(), Some("10.20.0.0/24"));
    assert_eq!(summary.synapse_dns_zone.as_deref(), Some("synapse.cynic"));
    assert_eq!(
        summary.provider_fallback_planes,
        vec!["proxmox_guest".to_string()]
    );
    assert_eq!(route.cidr, "10.10.10.0/24");
    assert_eq!(route.gateway.as_deref(), Some("10.10.10.1"));
}

#[test]
fn fetches_provider_fallback_service_checks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let adapter = LocalRegistryAdapter::new(write_registry(dir.path()));

    let checks = adapter
        .fetch_provider_fallback_service_checks()
        .expect("checks");

    assert!(checks.iter().any(|item| {
        item.node == "builder" && item.service == "ssh" && item.host == "builder.synapse.cynic"
    }));
    assert!(checks.iter().any(|item| {
        item.node == "kernel"
            && item.service == "cynic-kernel"
            && item.host == "cynic-kernel-primary.synapse.cynic"
    }));
}

#[test]
fn resolves_node_with_synapse_or_management_when_local_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("registry.json");
    fs::write(
        &path,
        r#"{
          "network_policy": {
            "canonical_service_plane": "synapse",
            "planes": {
              "synapse": { "cidr": "10.20.0.0/24", "dns_zone": "synapse.cynic", "canonical": true }
            }
          },
          "nodes": [
            {
              "name": "storage",
              "role": "storage_primary",
              "status": "active",
              "network": { "synapse": "10.20.0.103" },
              "services": [
                { "name": "surrealdb", "port": 8000 }
              ]
            },
            {
              "name": "cortex",
              "role": "cortex_host",
              "status": "active",
              "network": { "management": "192.168.100.2" },
              "services": [
                { "name": "cynic-kernel", "port": 3030 }
              ]
            }
          ]
        }"#,
    )
    .expect("write registry");

    let adapter = LocalRegistryAdapter::new(path);
    let checks = adapter
        .fetch_provider_fallback_service_checks()
        .expect("checks");

    assert!(checks.iter().any(|item| item.node == "storage"
        && item.service == "surrealdb"
        && item.ip == "10.20.0.103"));
    assert!(checks.iter().any(|item| item.node == "cortex"
        && item.service == "cynic-kernel"
        && item.ip == "192.168.100.2"));
}

#[test]
fn filters_planned_and_optional_services_from_checks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("registry.json");
    fs::write(
        &path,
        r#"{
          "network_policy": {
            "canonical_service_plane": "synapse",
            "planes": {
              "synapse": { "cidr": "10.20.0.0/24", "dns_zone": "synapse.cynic", "canonical": true }
            }
          },
          "nodes": [
            {
              "name": "forgejo",
              "role": "forge_primary",
              "status": "active",
              "network": { "synapse": "10.10.10.101" },
              "services": [
                { "name": "ssh", "port": 22 },
                { "name": "forgejo-backend", "port": 3000 },
                { "name": "forgejo-api-tls", "port": 443, "status": "planned" },
                { "name": "optional-metrics", "port": 9090, "optional": true }
              ]
            }
          ]
        }"#,
    )
    .expect("write registry");

    let adapter = LocalRegistryAdapter::new(path);
    let checks = adapter
        .fetch_provider_fallback_service_checks()
        .expect("checks");

    assert_eq!(checks.len(), 2);
    assert!(checks.iter().any(|item| item.service == "ssh"));
    assert!(checks.iter().any(|item| item.service == "forgejo-backend"));
    assert!(!checks.iter().any(|item| item.service == "forgejo-api-tls"));
    assert!(!checks.iter().any(|item| item.service == "optional-metrics"));
}
