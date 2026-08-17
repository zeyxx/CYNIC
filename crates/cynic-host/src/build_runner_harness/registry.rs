use cynic_protocol::{parse_harness_scenarios, HarnessScenario, HarnessScenarioId};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub fn read_registry(registry_path: &str) -> Result<Value, String> {
    let path = Path::new(registry_path);
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed_to_read_registry_file:{err}"))?;
    serde_json::from_str(&raw)
        .map_err(|err| format!("invalid_registry_json:{err}"))
}

pub fn load_harness_scenarios(registry_path: &str) -> Result<Vec<HarnessScenario>, String> {
    let registry = read_registry(registry_path)?;
    load_harness_scenarios_from_value(&registry)
}

pub fn load_harness_scenarios_from_value(registry: &Value) -> Result<Vec<HarnessScenario>, String> {
    let nodes = registry
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry_nodes_must_be_array".to_string())?;

    let builder_node = nodes.iter().find(|node| {
        node.get("role")
            .and_then(Value::as_str)
            .is_some_and(|role| role == "build_runner")
            || node
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| id == "cynic-builder")
    });

    if let Some(node) = builder_node {
        if let Some(scenarios_value) = node.get("harness_scenarios") {
            return parse_harness_scenarios(scenarios_value);
        }
    }

    if let Some(scenarios_value) = registry.get("harness_scenarios") {
        return parse_harness_scenarios(scenarios_value);
    }

    // Default canonical scenario suite if unspecified in minimal test fixture
    Ok(vec![
        HarnessScenario::closed_lab_vm(),
        HarnessScenario::lxc_clone_conformance(),
        HarnessScenario::builder_staging_observe(),
        HarnessScenario::builder_canary_apply(),
        HarnessScenario::failure_recovery(),
        HarnessScenario::proof_replay(),
    ])
}
