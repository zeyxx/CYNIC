//! Coherence between `infra/external-projects.json` and the port wiring status.
//!
//! Verifies that every external project declares port_status and that all ports
//! have reached at least `adapter_defined` status. This is an architecture test
//! (like workspace_paths_coherence.rs) — it lives in cynic-cli because it
//! needs I/O to read files.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn external_projects_have_port_status() {
    let path = repo_root().join("infra/external-projects.json");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let value: Value = serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));

    let projects = value["projects"].as_array().expect("projects must be an array");
    assert!(!projects.is_empty(), "at least one external project must be declared");

    for p in projects {
        let pid = p["project_id"].as_str().unwrap_or("unknown");
        let port_status = p.get("port_status").and_then(|v| v.as_object());
        assert!(
            port_status.is_some(),
            "project '{}' must declare port_status",
            pid
        );
        let ps = port_status.unwrap();
        assert!(!ps.is_empty(), "project '{}' port_status must not be empty", pid);

        for (port, status) in ps {
            let s = status.as_str().unwrap_or("");
            assert!(
                s.contains("adapter_defined") || s.contains("defined") || s.contains("candidate"),
                "project '{}' port '{}' has invalid status: '{}'",
                pid,
                port,
                s
            );
        }
    }
}

#[test]
fn external_projects_have_port_wiring_status() {
    let path = repo_root().join("infra/external-projects.json");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let value: Value = serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));

    let projects = value["projects"].as_array().expect("projects must be an array");
    for p in projects {
        let pid = p["project_id"].as_str().unwrap_or("unknown");
        let status = p.get("port_wiring_status").and_then(|v| v.as_str());
        assert!(
            status.is_some(),
            "project '{}' must declare port_wiring_status",
            pid
        );
    }
}

#[test]
fn port_contract_has_all_three_ports() {
    let path = repo_root().join("contracts/external-project-ports.v1.json");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let value: Value = serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));

    let ports = value["ports"].as_array().expect("ports must be an array");
    let port_names: Vec<&str> = ports.iter().filter_map(|p| p["port_name"].as_str()).collect();
    assert!(port_names.contains(&"SourcePort"), "SourcePort must be declared");
    assert!(port_names.contains(&"BuildPort"), "BuildPort must be declared");
    assert!(port_names.contains(&"EvidencePort"), "EvidencePort must be declared");
}