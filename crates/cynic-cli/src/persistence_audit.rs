//! Persistence audit surface — declared vs observed guest boot persistence.
//!
//! Projection only: the rule lives in
//! [`cynic_lifecycle::domain::persistence_drift`], which is pure and carries
//! its own tests. This module reads the registry, collects the observed
//! state from a provider observation port, and renders the verdict. A
//! surface projects; it does not own.
//!
//! Schema: `cynic.cli.persistence_audit.v1`.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use serde::Deserialize;

use cynic_lifecycle::domain::persistence_drift::{
    DeclaredGuest, DeclaredPersistence, ObservedGuest, ObservedPersistence, PersistenceDrift,
    evaluate_persistence_drift,
};

pub const PERSISTENCE_AUDIT_SCHEMA: &str = "cynic.cli.persistence_audit.v1";

#[derive(Debug, Deserialize)]
struct Registry {
    #[serde(default)]
    nodes: Vec<RegistryNode>,
}

#[derive(Debug, Deserialize)]
struct RegistryNode {
    #[serde(default)]
    id: String,
    #[serde(default)]
    deploy_target: bool,
    /// Present only when the registry actually declares it. A guest whose
    /// registry entry omits this key is exactly the CT 102 case: the
    /// provider defaults it off and the guest vanishes at reboot.
    #[serde(default)]
    boot_persistence: Option<bool>,
    /// A node carrying a provider resource id is a hosted guest; one without
    /// it is the baremetal host, whose boot behaviour is not a guest flag.
    #[serde(default)]
    proxmox_id: Option<String>,
}

/// Declared side of the audit, straight from the registry.
fn declared_guests(registry: &Registry) -> Vec<DeclaredGuest> {
    registry
        .nodes
        .iter()
        .map(|node| DeclaredGuest {
            node_id: node.id.clone(),
            deploy_target: node.deploy_target,
            declared_persistence: match node.boot_persistence {
                Some(value) => DeclaredPersistence::Declared(value),
                None => DeclaredPersistence::Absent,
            },
            hosted: node.proxmox_id.is_some(),
        })
        .collect()
}

/// Registry file shape, read without inventing a default for a missing file.
fn read_registry(path: &str) -> Result<Registry, String> {
    let raw = std::fs::read_to_string(Path::new(path))
        .map_err(|error| format!("registry read failed: {path}: {error}"))?;
    serde_json::from_str(&raw).map_err(|error| format!("registry parse failed: {path}: {error}"))
}

/// Render the drift list as JSON-shaped values without a serde derive on the
/// domain types, so this surface stays a projection.
fn drifts_to_json(drifts: &[PersistenceDrift]) -> Vec<serde_json::Value> {
    drifts
        .iter()
        .map(|drift| {
            serde_json::json!({
                "node_id": drift.node_id,
                "kind": drift.kind,
                "evidence": drift.evidence,
            })
        })
        .collect()
}

/// Build the audit report from an already-collected observation.
///
/// Pure with respect to the domain: the same registry and the same observed
/// map always yield the same report, so the test suite can exercise the whole
/// projection without a Proxmox host.
pub fn build_report(
    registry: &Registry,
    observed: &BTreeMap<String, ObservedPersistence>,
) -> Report {
    let declared = declared_guests(registry);
    let observed_guests: Vec<ObservedGuest> = observed
        .iter()
        .map(|(node_id, observed_persistence)| ObservedGuest {
            node_id: node_id.clone(),
            observed_persistence: *observed_persistence,
        })
        .collect();

    let drifts = evaluate_persistence_drift(&declared, &observed_guests);
    let deploy_targets = declared
        .iter()
        .filter(|d| d.deploy_target && d.hosted)
        .count();
    let undeclared = declared
        .iter()
        .filter(|d| {
            d.deploy_target && d.hosted && d.declared_persistence == DeclaredPersistence::Absent
        })
        .count();

    Report {
        schema: PERSISTENCE_AUDIT_SCHEMA.to_string(),
        status: if drifts.is_empty() { "ok" } else { "drift" }.to_string(),
        mutates: false,
        deploy_target_guests: deploy_targets,
        guests_without_declared_persistence: undeclared,
        observed_guests: observed.len(),
        drifts: drifts_to_json(&drifts),
        residual_unknowns: vec![
            "Observed boot persistence comes from the provider observation port. \
             A guest absent from that map is reported as unobserved, never as passing."
                .to_string(),
        ],
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Report {
    pub schema: String,
    pub status: String,
    pub mutates: bool,
    pub deploy_target_guests: usize,
    pub guests_without_declared_persistence: usize,
    pub observed_guests: usize,
    pub drifts: Vec<serde_json::Value>,
    pub residual_unknowns: Vec<String>,
}

fn registry_path(override_path: &Option<String>) -> String {
    override_path
        .clone()
        .unwrap_or_else(|| "infra/registry.json".to_string())
}

fn observed_from_arg(raw: &Option<String>) -> BTreeMap<String, ObservedPersistence> {
    let Some(raw) = raw else {
        return BTreeMap::new();
    };
    serde_json::from_str::<BTreeMap<String, ObservedPersistence>>(raw).unwrap_or_default()
}

pub(crate) fn execute(
    json: bool,
    registry: &Option<String>,
    observed: &Option<String>,
) -> ExitCode {
    let path = registry_path(registry);
    let registry = match read_registry(&path) {
        Ok(registry) => registry,
        Err(error) => return crate::fail(error),
    };

    let observed_map = observed_from_arg(observed);
    let report = build_report(&registry, &observed_map);
    let blocked = report.status != "ok";

    if json {
        crate::emit_json(&report, blocked)
    } else {
        println!(
            "Persistence audit: {} deploy-target guests",
            report.deploy_target_guests
        );
        println!(
            "Observed: {} guests, {} without a declared persistence flag",
            report.observed_guests, report.guests_without_declared_persistence
        );
        for drift in &report.drifts {
            let node = drift.get("node_id").and_then(|v| v.as_str()).unwrap_or("?");
            let kind = drift.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
            let evidence = drift.get("evidence").and_then(|v| v.as_str()).unwrap_or("");
            println!("  [{kind}] {node}: {evidence}");
        }
        if blocked {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_from(raw: &str) -> Registry {
        serde_json::from_str(raw).expect("fixture must parse")
    }

    const FIXTURE: &str = r#"{
      "nodes": [
        {"id": "cynic-host-proxmox", "deploy_target": false},
        {"id": "cynic-builder", "deploy_target": true, "proxmox_id": "lxc/105", "boot_persistence": true},
        {"id": "cynic-agent-host", "deploy_target": true, "proxmox_id": "lxc/102", "boot_persistence": true}
      ]
    }"#;

    #[test]
    fn declared_registry_produces_no_drift_when_all_observed_persistent() {
        let registry = registry_from(FIXTURE);
        let observed: BTreeMap<String, ObservedPersistence> = [
            (
                "cynic-builder".to_string(),
                ObservedPersistence::BootPersistent,
            ),
            (
                "cynic-agent-host".to_string(),
                ObservedPersistence::BootPersistent,
            ),
        ]
        .into_iter()
        .collect();

        let report = build_report(&registry, &observed);

        assert_eq!(report.status, "ok");
        assert!(report.drifts.is_empty());
        assert_eq!(report.deploy_target_guests, 2);
    }

    #[test]
    fn guest_with_absent_declaration_and_non_persistent_observation_is_drift() {
        let registry = registry_from(
            r#"{"nodes":[{"id":"cynic-agent-host","deploy_target":true,"proxmox_id":"lxc/102"}]}"#,
        );
        let observed: BTreeMap<String, ObservedPersistence> = [(
            "cynic-agent-host".to_string(),
            ObservedPersistence::NotBootPersistent,
        )]
        .into_iter()
        .collect();

        let report = build_report(&registry, &observed);

        assert_eq!(report.status, "drift");
        assert_eq!(report.guests_without_declared_persistence, 1);
        assert_eq!(report.drifts.len(), 1);
        let evidence = report.drifts[0]["evidence"].as_str().unwrap_or_default();
        assert!(evidence.contains("no onboot key"), "got: {evidence}");
    }

    #[test]
    fn guest_with_declared_flag_but_non_persistent_provider_is_still_drift() {
        let registry = registry_from(FIXTURE);
        let observed: BTreeMap<String, ObservedPersistence> = [
            (
                "cynic-builder".to_string(),
                ObservedPersistence::NotBootPersistent,
            ),
            (
                "cynic-agent-host".to_string(),
                ObservedPersistence::BootPersistent,
            ),
        ]
        .into_iter()
        .collect();

        let report = build_report(&registry, &observed);

        assert_eq!(report.status, "drift");
        assert_eq!(report.drifts[0]["node_id"].as_str(), Some("cynic-builder"));
    }

    #[test]
    fn missing_observation_is_unobserved_never_passing_silence() {
        let registry = registry_from(FIXTURE);

        let report = build_report(&registry, &BTreeMap::new());

        assert_eq!(report.status, "drift");
        assert_eq!(report.observed_guests, 0);
        assert_eq!(report.drifts.len(), 2, "both deploy targets are unobserved");
        let kinds: Vec<&str> = report
            .drifts
            .iter()
            .filter_map(|d| d["kind"].as_str())
            .collect();
        assert!(kinds.iter().all(|k| *k == "unobserved"), "got {kinds:?}");
    }

    #[test]
    fn baremetal_host_is_never_counted_as_a_deploy_target_guest() {
        let registry = registry_from(
            r#"{"nodes":[{"id":"cynic-host-proxmox","deploy_target":true,"boot_persistence":true}]}"#,
        );

        let report = build_report(&registry, &BTreeMap::new());

        assert_eq!(report.deploy_target_guests, 0);
        assert_eq!(report.status, "ok");
    }

    #[test]
    fn report_is_json_serialisable_for_projection() {
        let registry = registry_from(FIXTURE);
        let observed: BTreeMap<String, ObservedPersistence> = [(
            "cynic-agent-host".to_string(),
            ObservedPersistence::NotBootPersistent,
        )]
        .into_iter()
        .collect();

        let report = build_report(&registry, &observed);
        let encoded = serde_json::to_string(&report).expect("report must serialise");

        assert!(encoded.contains(PERSISTENCE_AUDIT_SCHEMA));
        assert!(encoded.contains("\"status\":\"drift\""));
    }

    #[test]
    fn unreadable_registry_fails_rather_than_reporting_clean() {
        let result = read_registry("/nonexistent/registry.json");
        assert!(
            result.is_err(),
            "a missing registry must be an error, not a pass"
        );
    }
}
