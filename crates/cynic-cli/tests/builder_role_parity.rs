//! Every node carrying the build_runner role meets the same bar.
//!
//! THE DIVERGENCE THIS LOCKS. compute-01 executed builds for a day with
//! no declared capabilities, no sccache, and a stale toolchain symlink —
//! while the builder carried all three. The submitter picked the first
//! healthy runner and the fleet silently idled its strongest machine
//! (8c/16G) behind the weaker one, then failed it for a missing wrapper
//! the registry never required.
//!
//! These tests read the COMMITTED registry, so a runner that drifts
//! below the role bar fails here rather than at 2am as a cryptic spawn
//! error on a machine nobody declared anything for.

use std::path::Path;

fn registry_value() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../infra/registry.json")
        .canonicalize()
        .expect("registry.json is reachable from the crate manifest");
    let bytes = std::fs::read(&path).expect("registry.json is readable");
    serde_json::from_slice(&bytes).expect("registry.json is valid JSON")
}

fn build_runner_nodes() -> Vec<serde_json::Value> {
    let registry = registry_value();
    let nodes = registry
        .get("nodes")
        .and_then(serde_json::Value::as_array)
        .expect("the registry declares nodes");
    nodes
        .iter()
        .filter(|node| node.get("role").and_then(serde_json::Value::as_str) == Some("build_runner"))
        .cloned()
        .collect()
}

fn node_id(node: &serde_json::Value) -> &str {
    node.get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("<unknown>")
}

#[test]
fn the_fleet_has_at_least_one_build_runner() {
    assert!(
        !build_runner_nodes().is_empty(),
        "the pool resolves nothing without a build_runner"
    );
}

#[test]
fn every_build_runner_declares_hardware_capabilities() {
    for node in build_runner_nodes() {
        let caps = node
            .get("capabilities")
            .unwrap_or_else(|| panic!("{} declares no capabilities", node_id(&node)));
        let cpu = caps
            .get("cpu_cores")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let ram = caps
            .get("ram_gb")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let gpu = caps
            .get("gpu")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        assert!(cpu > 0, "{} declares no cpu_cores", node_id(&node));
        assert!(ram > 0, "{} declares no ram_gb", node_id(&node));
        assert!(!gpu.trim().is_empty(), "{} declares no gpu", node_id(&node));
    }
}

#[test]
fn every_build_runner_requires_the_toolchain_triple() {
    for node in build_runner_nodes() {
        let tools = node
            .pointer("/pool/required_tools")
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| panic!("{} declares no pool.required_tools", node_id(&node)));
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(serde_json::Value::as_str))
            .collect();
        for required in ["git", "cargo", "rustc"] {
            assert!(
                names.contains(&required),
                "{} misses required tool {required}",
                node_id(&node)
            );
        }
        let cargo = tools
            .iter()
            .find(|t| t.get("name").and_then(serde_json::Value::as_str) == Some("cargo"));
        let min = cargo
            .and_then(|t| t.get("min_version"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        assert_eq!(
            min,
            "1.95.0",
            "{} pins a different cargo floor",
            node_id(&node)
        );
    }
}

#[test]
fn every_build_runner_is_enrolled() {
    for node in build_runner_nodes() {
        assert!(
            node.get("enrollment").is_some(),
            "{} has no enrollment record",
            node_id(&node)
        );
    }
}

/// The disk gate reads `build_lifecycle` per runner. A runner without one has
/// its gate silently off (`minimum_available_bytes_for_runner` yields `None`),
/// which is the same silent gap as a runner with no declared capabilities.
#[test]
fn every_build_runner_declares_a_build_lifecycle() {
    for node in build_runner_nodes() {
        assert!(
            node.get("build_lifecycle").is_some(),
            "{} declares no build_lifecycle: its disk gate would be off",
            node_id(&node)
        );
    }
}
