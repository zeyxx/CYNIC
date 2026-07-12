use cynic_protocol::{
    RUNTIME_EXECUTION_SUITE_SCHEMA, RuntimeExecutionPlan, RuntimeExecutionSuite,
    RuntimeProofBundle, RuntimeRegistrySummary, build_runtime_execution_plan_from_gates,
    runtime_execution_layers, runtime_initial_observed_gates,
};
use serde_json::Value;

pub(crate) fn build_runtime_execution_suite(
    registry_path: &str,
    profile: &str,
) -> Result<RuntimeExecutionSuite, String> {
    let raw = std::fs::read_to_string(registry_path)
        .map_err(|error| format!("failed to read registry {registry_path}: {error}"))?;
    let registry: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse registry {registry_path}: {error}"))?;
    let summary = registry_summary(&registry);

    Ok(RuntimeExecutionSuite {
        schema: RUNTIME_EXECUTION_SUITE_SCHEMA,
        status: "planned",
        registry_path: registry_path.to_string(),
        profile: profile.to_string(),
        rule: "Each layer has an explicit read-only probe, optional dry-run, optional apply, and an exit gate before the next layer.",
        source_prisms: vec![
            "cloud_characteristics",
            "iaas_paas_saas_shared_responsibility",
            "public_private_hybrid_community_deployment_models",
            "cloud_reference_actors_consumer_provider_broker_carrier_auditor",
            "storage_redundancy_encryption_access_control",
        ],
        cloud_characteristics: vec![
            "on_demand_self_service",
            "broad_network_access",
            "resource_pooling",
            "rapid_elasticity",
            "measured_service",
            "sla_or_exit_gate",
        ],
        deployment_models: vec!["private", "public", "hybrid", "community"],
        service_models: vec!["IaaS", "PaaS", "SaaS"],
        registry_summary: summary,
        layers: runtime_execution_layers(registry_path),
    })
}

pub(crate) fn build_runtime_execution_plan(
    registry_path: &str,
    profile: &str,
    proof_path: Option<&str>,
) -> Result<RuntimeExecutionPlan, String> {
    let mut observed_gates = runtime_initial_observed_gates();
    if let Some(proof_path) = proof_path {
        observed_gates.extend(load_observed_gates(proof_path)?);
    }
    build_runtime_execution_plan_with_observed_gates(registry_path, profile, observed_gates)
}

pub(crate) fn build_runtime_execution_plan_with_observed_gates(
    registry_path: &str,
    profile: &str,
    observed_gates: Vec<String>,
) -> Result<RuntimeExecutionPlan, String> {
    let raw = std::fs::read_to_string(registry_path)
        .map_err(|error| format!("failed to read registry {registry_path}: {error}"))?;
    let registry: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse registry {registry_path}: {error}"))?;
    let summary = registry_summary(&registry);
    Ok(build_runtime_execution_plan_from_gates(
        registry_path,
        profile,
        summary,
        observed_gates,
    ))
}

fn registry_summary(registry: &Value) -> RuntimeRegistrySummary {
    let nodes = registry
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut roles = nodes
        .iter()
        .filter_map(|node| node.get("role").and_then(Value::as_str))
        .filter(|role| !role.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    roles.sort();
    roles.dedup();
    let service_count = nodes
        .iter()
        .flat_map(|node| node.get("services").and_then(Value::as_array))
        .map(Vec::len)
        .sum();

    RuntimeRegistrySummary {
        node_count: nodes.len(),
        roles,
        service_count,
        dns_zone: registry
            .pointer("/network_policy/planes/synapse/dns_zone")
            .and_then(Value::as_str)
            .map(str::to_string),
        provider_cidr: registry
            .pointer("/network_policy/planes/proxmox_guest/cidr")
            .and_then(Value::as_str)
            .map(str::to_string),
    }
}

fn load_observed_gates(proof_path: &str) -> Result<Vec<String>, String> {
    let raw = std::fs::read_to_string(proof_path)
        .map_err(|error| format!("failed to read runtime proof bundle {proof_path}: {error}"))?;
    let bundle: RuntimeProofBundle = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse runtime proof bundle {proof_path}: {error}"))?;
    Ok(bundle.observed_gates())
}
