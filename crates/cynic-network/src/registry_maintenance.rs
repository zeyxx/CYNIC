//! Dynamic Registry Maintenance & Convergence Domain for CYNIC
//! Ingests NetworkDiscoveryReports, updates the "observed" section of registry.json,
//! and triggers bounded auto-reconciliation actions without manual human intervention.

use crate::discovery::{NetworkDiscoveryReport, NodeReachability};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const REGISTRY_UPDATE_REPORT_SCHEMA: &str = "cynic.network.registry_update_report.v1";
pub const REGISTRY_SYNC_REPORT_SCHEMA: &str = "cynic.network.registry_sync_report.v1";
pub const REGISTRY_RECONCILIATION_REPORT_SCHEMA: &str =
    "cynic.network.registry_reconciliation_report.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryReconciliationStatus {
    Ok,
    DriftDetected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryReconciliationReport {
    pub schema: String,
    pub status: RegistryReconciliationStatus,
    pub observer_node_id: String,
    pub registry_path: String,
    pub nodes_evaluated: usize,
    pub nodes_updated: usize,
    pub drift_detected: bool,
    pub drift_summary: Vec<String>,
    pub applied: bool,
    pub observed_nodes: Option<serde_json::Value>,
    pub observed_at_unix_ms: u64,
}

pub fn build_reconciliation_report(
    observer_node_id: impl Into<String>,
    registry_path: impl Into<String>,
    nodes_evaluated: usize,
    nodes_updated: usize,
    drift_detected: bool,
    drift_summary: Vec<String>,
    applied: bool,
    observed_nodes: Option<serde_json::Value>,
    observed_at_unix_ms: u64,
) -> RegistryReconciliationReport {
    RegistryReconciliationReport {
        schema: REGISTRY_RECONCILIATION_REPORT_SCHEMA.to_string(),
        status: if drift_detected {
            RegistryReconciliationStatus::DriftDetected
        } else {
            RegistryReconciliationStatus::Ok
        },
        observer_node_id: observer_node_id.into(),
        registry_path: registry_path.into(),
        nodes_evaluated,
        nodes_updated,
        drift_detected,
        drift_summary,
        applied,
        observed_nodes,
        observed_at_unix_ms,
    }
}
/// Backups retained beside the runtime registry copy after a sync.
pub const MAX_REGISTRY_SYNC_BACKUPS: usize = 5;

/// Applies discovered network observations directly to a registry JSON Value
pub fn reconcile_registry_observations(
    registry: &mut Value,
    discovery: &NetworkDiscoveryReport,
) -> Result<usize, String> {
    let root = registry
        .as_object_mut()
        .ok_or_else(|| "registry root must be a JSON object".to_string())?;

    let observed = root
        .entry("observed")
        .or_insert_with(|| json!({"nodes": {}}));

    let observed_nodes = observed
        .get_mut("nodes")
        .map(|n| {
            if !n.is_object() {
                *n = json!({});
            }
            n
        })
        .ok_or_else(|| "observed.nodes must be a JSON object".to_string())?
        .as_object_mut()
        .ok_or_else(|| "observed.nodes must be a JSON object".to_string())?;

    let mut updated_count = 0;
    for (node_id, node) in &discovery.nodes {
        let entry = json!({
            "status": match node.reachability {
                NodeReachability::Reachable { .. } => "observed_active",
                NodeReachability::Degraded { .. } => "observed_degraded",
                NodeReachability::Unreachable { .. } => "observed_unreachable",
            },
            "synapse_ip": node.synapse_ip,
            "wireguard_ip": node.wireguard_ip,
            "direct_ip": node.direct_ip,
            "last_observed_unix_ms": node.observed_at_unix_ms,
            "services_count": node.services.len(),
        });
        observed_nodes.insert(node_id.clone(), entry);
        updated_count += 1;
    }

    Ok(updated_count)
}

/// Merge outcome of syncing declared registry content into a runtime copy.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RegistrySyncReport {
    pub schema: &'static str,
    pub created: bool,
    pub nodes_added: Vec<String>,
    pub nodes_removed: Vec<String>,
    pub planes_changed: bool,
    pub zone_changed: bool,
    pub stale_observed_nodes: Vec<String>,
}

/// Replace the declared content of a runtime registry copy with the
/// source registry, preserving the runtime-only `observed` section.
///
/// Rules (see docs/specs/registry-sync-surface.md): wholesale declared
/// replacement; `observed` preserved byte-identical and shape-validated
/// (object with a `nodes` object — malformed blocks the sync); observed
/// nodes missing from declared are kept and reported, never deleted;
/// schema version divergence between source and an existing target
/// blocks the sync; an absent target is created, not an error.
pub fn apply_registry_sync(
    source: &Value,
    target: &mut Value,
) -> Result<RegistrySyncReport, String> {
    let source_root = source
        .as_object()
        .ok_or_else(|| "sync source registry root must be a JSON object".to_string())?;
    let source_version = source_root
        .get("version")
        .and_then(|version| {
            if let Some(number) = version.as_u64() {
                return Some(number.to_string());
            }
            version.as_str().map(str::to_string)
        })
        .ok_or_else(|| "sync source registry has no version".to_string())?;
    let created = target.is_null();
    if created {
        *target = json!({});
    }
    let target_root = target
        .as_object_mut()
        .ok_or_else(|| "sync target registry root must be a JSON object".to_string())?;
    if !created {
        let target_version = target_root
            .get("version")
            .and_then(|version| {
                if let Some(number) = version.as_u64() {
                    return Some(number.to_string());
                }
                version.as_str().map(str::to_string)
            })
            .ok_or_else(|| "sync target registry has no version".to_string())?;
        if target_version != source_version {
            return Err(format!(
                "sync schema version diverged: source {source_version} vs target {target_version}"
            ));
        }
    }

    let declared_names = node_names(source_root);
    let previous_names = node_names(target_root);
    let nodes_added = declared_names
        .iter()
        .filter(|name| !previous_names.contains(name))
        .cloned()
        .collect::<Vec<_>>();
    let nodes_removed = previous_names
        .iter()
        .filter(|name| !declared_names.contains(name))
        .cloned()
        .collect::<Vec<_>>();
    let planes_changed = source_root.get("network_policy") != target_root.get("network_policy");
    let zone_changed = service_dns_zone(source_root) != service_dns_zone(target_root);

    let mut stale_observed_nodes = Vec::new();
    if let Some(observed) = target_root.get("observed") {
        let nodes = observed
            .get("nodes")
            .and_then(|nodes| nodes.as_object())
            .ok_or_else(|| "sync target observed.nodes must be a JSON object".to_string())?;
        for key in nodes.keys() {
            if !declared_names.contains(key) {
                stale_observed_nodes.push(key.clone());
            }
        }
    }

    let preserved = target_root.remove("observed");
    target_root.clear();
    for (key, value) in source_root {
        target_root.insert(key.clone(), value.clone());
    }
    if let Some(observed) = preserved {
        target_root.insert("observed".to_string(), observed);
    }

    Ok(RegistrySyncReport {
        schema: REGISTRY_SYNC_REPORT_SCHEMA,
        created,
        nodes_added,
        nodes_removed,
        planes_changed,
        zone_changed,
        stale_observed_nodes,
    })
}

fn node_names(root: &serde_json::Map<String, Value>) -> Vec<String> {
    root.get("nodes")
        .and_then(|nodes| nodes.as_array())
        .map(|nodes| {
            nodes
                .iter()
                .filter_map(|node| {
                    node.get("name")
                        .and_then(|name| name.as_str())
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn service_dns_zone(root: &serde_json::Map<String, Value>) -> Option<String> {
    root.get("network_policy")?
        .get("planes")?
        .get("service")?
        .get("dns_zone")?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::DiscoveredNode;
    use std::collections::BTreeMap;

    #[test]
    fn reconciles_discovered_nodes_into_registry_observed_section() {
        let mut registry = json!({
            "schema": "cynic.network.registry.v1",
            "nodes": [
                {"id": "cynic-builder", "name": "cynic-builder"}
            ]
        });

        let mut nodes = BTreeMap::new();
        nodes.insert(
            "cynic-builder".to_string(),
            DiscoveredNode {
                node_id: "cynic-builder".to_string(),
                direct_ip: None,
                synapse_ip: Some("10.10.10.119".to_string()),
                wireguard_ip: Some("10.20.0.5".to_string()),
                reachability: NodeReachability::Reachable {
                    latency_ms: 1,
                    protocol: "tcp".to_string(),
                },
                services: vec![],
                observed_at_unix_ms: 123456789,
            },
        );

        let report = NetworkDiscoveryReport {
            schema: "cynic.network.discovery_report.v1".to_string(),
            observer_node_id: "cortex".to_string(),
            nodes,
            drift_detected: false,
            drift_summary: vec![],
            observed_at_unix_ms: 123456789,
        };

        let count = reconcile_registry_observations(&mut registry, &report).expect("reconcile ok");
        assert_eq!(count, 1);
        assert_eq!(
            registry["observed"]["nodes"]["cynic-builder"]["status"],
            "observed_active"
        );
        assert_eq!(
            registry["observed"]["nodes"]["cynic-builder"]["synapse_ip"],
            "10.10.10.119"
        );
    }

    fn sync_source() -> Value {
        json!({
            "version": 1,
            "network_policy": {
                "planes": {"service": {"cidr": "10.10.10.0/24", "dns_zone": "service.cynic"}}
            },
            "nodes": [{"name": "a"}, {"name": "b"}]
        })
    }

    #[test]
    fn sync_preserves_observed_and_reports_diff() {
        let source = sync_source();
        let mut target = json!({
            "version": 1,
            "network_policy": {
                "planes": {"synapse": {"cidr": "10.10.10.0/24"}}
            },
            "nodes": [{"name": "a"}, {"name": "ghost"}],
            "observed": {"nodes": {"a": {"status": "x"}, "ghost": {"status": "y"}}}
        });
        let report = apply_registry_sync(&source, &mut target).expect("sync ok");
        assert!(!report.created);
        assert_eq!(report.nodes_added, vec!["b".to_string()]);
        assert_eq!(report.nodes_removed, vec!["ghost".to_string()]);
        assert!(report.planes_changed);
        assert!(report.zone_changed);
        assert_eq!(report.stale_observed_nodes, vec!["ghost".to_string()]);
        assert_eq!(target["observed"]["nodes"]["a"]["status"], "x");
        assert_eq!(target["observed"]["nodes"]["ghost"]["status"], "y");
        assert_eq!(target["nodes"].as_array().map(Vec::len), Some(2));
    }

    #[test]
    fn sync_creates_absent_target() {
        let source = sync_source();
        let mut target = Value::Null;
        let report = apply_registry_sync(&source, &mut target).expect("sync ok");
        assert!(report.created);
        assert_eq!(target["nodes"].as_array().map(Vec::len), Some(2));
    }

    #[test]
    fn sync_rejects_version_divergence_and_malformed_observed() {
        let source = sync_source();
        let mut diverged = json!({"version": 2, "nodes": []});
        apply_registry_sync(&source, &mut diverged).expect_err("version must block");
        let mut malformed = json!({
            "version": 1,
            "nodes": [],
            "observed": {"nodes": [1, 2]}
        });
        apply_registry_sync(&source, &mut malformed).expect_err("shape must block");
    }

    #[test]
    fn sync_is_idempotent() {
        let source = sync_source();
        let mut target = json!({"version": 1, "nodes": [{"name": "a"}]});
        apply_registry_sync(&source, &mut target).expect("first");
        let report = apply_registry_sync(&source, &mut target).expect("second");
        assert!(report.nodes_added.is_empty());
        assert!(report.nodes_removed.is_empty());
        assert!(!report.planes_changed);
    }
}
