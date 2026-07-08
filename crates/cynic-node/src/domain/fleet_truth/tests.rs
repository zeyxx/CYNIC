use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

use super::*;

fn burst_profile() -> BTreeMap<String, Value> {
    BTreeMap::from([("class".to_string(), json!("burst"))])
}

#[test]
fn active_node_without_observation_is_critical_and_stale() {
    let report = build_report(
        &[Node {
            name: "cynic-vm-anvil".to_string(),
            role: "build_runner".to_string(),
            status: "active".to_string(),
            constellation: None,
            aliases: Vec::new(),
            host: None,
            proxmox_id: None,
            energy_profile: burst_profile(),
        }],
        &[],
        &BTreeSet::from(["organ-fleet".to_string()]),
        &BTreeSet::from(["fleet".to_string()]),
    );

    assert_eq!(report.summary.status, "degraded");
    assert_eq!(report.summary.critical_targets, vec!["cynic-vm-anvil"]);
    assert_eq!(report.effective[0].effective_status, "stale");
    assert!(
        report
            .anomalies
            .iter()
            .any(|anomaly| anomaly.kind == "active_without_runtime_proof")
    );
}

#[test]
fn observed_identity_satisfies_active_node() {
    let report = build_report(
        &[Node {
            name: "cynic-vm-anvil".to_string(),
            role: "build_runner".to_string(),
            status: "active".to_string(),
            constellation: None,
            aliases: vec!["forge".to_string()],
            host: None,
            proxmox_id: Some("qemu/102".to_string()),
            energy_profile: burst_profile(),
        }],
        &[ObservedResource {
            provider: "proxmox".to_string(),
            name: "cynic-vm-anvil".to_string(),
            status: "running".to_string(),
            resource_id: Some("qemu/102".to_string()),
            kind: Some("vm".to_string()),
            constellation: None,
        }],
        &BTreeSet::from(["organ-fleet".to_string()]),
        &BTreeSet::from(["fleet".to_string()]),
    );

    assert_eq!(report.summary.status, "ok");
    assert_eq!(report.effective[0].effective_status, "active");
    assert!(
        report
            .anomalies
            .iter()
            .all(|anomaly| anomaly.kind != "active_without_runtime_proof")
    );
}

#[test]
fn watchdog_constellation_evidence_flows_into_effective_summary() {
    let report = build_report(
        &[Node {
            name: "cynic-vm-v2".to_string(),
            role: "kernel_primary".to_string(),
            status: "active".to_string(),
            constellation: Some("kernel".to_string()),
            aliases: Vec::new(),
            host: None,
            proxmox_id: None,
            energy_profile: burst_profile(),
        }],
        &[ObservedResource {
            provider: "cynic-node-watchdog".to_string(),
            name: "cynic-vm-v2".to_string(),
            status: "running".to_string(),
            resource_id: None,
            kind: Some("node_watchdog".to_string()),
            constellation: Some("kernel".to_string()),
        }],
        &BTreeSet::from(["organ-fleet".to_string()]),
        &BTreeSet::from(["fleet".to_string()]),
    );

    assert_eq!(
        report.effective[0].provider.as_deref(),
        Some("cynic-node-watchdog")
    );
    assert_eq!(
        report.effective[0].observed_constellation.as_deref(),
        Some("kernel")
    );
    assert_eq!(
        report.summary.constellations.observed.get("kernel"),
        Some(&1)
    );
}
