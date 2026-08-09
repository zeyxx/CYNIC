fn validate_api_operation_geometry(
    operations: &std::collections::BTreeSet<String>,
    topology: &serde_json::Value,
    topology_operations: &std::collections::BTreeSet<String>,
    allowed_layers: &std::collections::BTreeSet<String>,
    allowed_mutation_levels: &std::collections::BTreeSet<String>,
    allowed_statuses: &std::collections::BTreeSet<String>,
    topology_budget: &serde_json::Map<String, serde_json::Value>,
) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();
    let empty = serde_json::Map::new();
    let topo_ops = topology
        .get("operations")
        .and_then(|value| value.as_object())
        .unwrap_or(&empty);
    let mut by_root: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for (operation, contract) in topo_ops {
        let family = contract
            .get("canonical_family")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        by_root
            .entry(api_operation_root(operation))
            .or_default()
            .insert(family);
    }
    let root_splits = by_root.values().filter(|families| families.len() > 1).count();
    let transitional_count = topo_ops
        .values()
        .filter(|contract| contract.get("status").and_then(|value| value.as_str()) == Some("transitional"))
        .count();
    let legacy_count = topo_ops
        .values()
        .filter(|contract| contract.get("status").and_then(|value| value.as_str()) == Some("legacy"))
        .count();
    let max_root_splits = topology_budget.get("max_root_splits").and_then(|value| value.as_u64());
    let max_transitional = topology_budget
        .get("max_transitional_operations")
        .and_then(|value| value.as_u64());
    let max_legacy = topology_budget
        .get("max_legacy_operations")
        .and_then(|value| value.as_u64());
    if max_root_splits.is_none_or(|budget| root_splits as u64 > budget) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_topology_root_split_budget_exceeded",
            "path": "axioms/api-topology-v1.json",
            "detail": format!("found {root_splits} root splits, budget {max_root_splits:?}."),
        }));
    }
    if max_transitional.is_none_or(|budget| transitional_count as u64 > budget) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_topology_transitional_budget_exceeded",
            "path": "axioms/api-topology-v1.json",
            "detail": format!("found {transitional_count} transitional operations, budget {max_transitional:?}."),
        }));
    }
    if max_legacy.is_none_or(|budget| legacy_count as u64 > budget) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_topology_legacy_budget_exceeded",
            "path": "axioms/api-topology-v1.json",
            "detail": format!("found {legacy_count} legacy operations, budget {max_legacy:?}."),
        }));
    }
    for operation in operations.difference(topology_operations) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_topology_operation_missing",
            "path": "axioms/api-topology-v1.json",
            "detail": format!("missing topology entry for OpenAPI operation: {operation}."),
        }));
    }
    for operation in topology_operations.difference(operations) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_topology_operation_stale",
            "path": "axioms/api-topology-v1.json",
            "detail": format!("stale topology entry absent from OpenAPI: {operation}."),
        }));
    }

    let required_fields = [
        "layer",
        "mutation_level",
        "agent_safe_default",
        "canonical_family",
        "status",
    ];
    let dangerous = ["deployment", "destructive"];
    for (operation, contract) in topo_ops {
        let Some(contract_object) = contract.as_object() else {
            continue;
        };
        let missing_fields: Vec<&str> = required_fields
            .iter()
            .copied()
            .filter(|field| !contract_object.contains_key(*field))
            .collect();
        if !missing_fields.is_empty() {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_topology_operation_metadata_missing",
                "path": operation,
                "detail": format!("missing fields {missing_fields:?}."),
            }));
            continue;
        }
        let layer = contract.get("layer").and_then(|value| value.as_str()).unwrap_or_default();
        let mutation_level = contract
            .get("mutation_level")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let status = contract.get("status").and_then(|value| value.as_str()).unwrap_or_default();
        let agent_safe_default = contract.get("agent_safe_default");
        let canonical_family = contract
            .get("canonical_family")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        if !allowed_layers.contains(layer) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_topology_operation_layer_invalid",
                "path": operation,
                "detail": format!("invalid layer {layer:?}."),
            }));
        }
        if !allowed_mutation_levels.contains(mutation_level) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_topology_operation_mutation_level_invalid",
                "path": operation,
                "detail": format!("invalid mutation_level {mutation_level:?}."),
            }));
        }
        if !allowed_statuses.contains(status) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_topology_operation_status_invalid",
                "path": operation,
                "detail": format!("invalid status {status:?}."),
            }));
        }
        if !agent_safe_default.is_some_and(serde_json::Value::is_boolean) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_topology_agent_safe_default_invalid",
                "path": operation,
                "detail": "agent_safe_default must be boolean.",
            }));
        }
        if !canonical_family.starts_with('/') {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_topology_canonical_family_invalid",
                "path": operation,
                "detail": "canonical_family must be an absolute path family.",
            }));
        }
        if dangerous.contains(&mutation_level)
            && contract.get("agent_safe_default").and_then(|value| value.as_bool()) != Some(false)
        {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_topology_dangerous_mutation_agent_safe",
                "path": operation,
                "detail": "dangerous mutation cannot be agent safe by default.",
            }));
        }
        if mutation_level == "deployment" && layer != "metabolism" && canonical_family != "/lifecycle" {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_topology_deployment_not_metabolism",
                "path": operation,
                "detail": "deployment mutation must be metabolism or point to /lifecycle.",
            }));
        }
        if layer == "perception" && dangerous.contains(&mutation_level) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_topology_perception_mutates_dangerously",
                "path": operation,
                "detail": "perception cannot use dangerous mutation.",
            }));
        }
        let operation_path = operation.split_once(' ').map(|(_, path)| path).unwrap_or_default();
        if ["compat", "transitional"].contains(&status) && canonical_family == operation_path {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_topology_compat_family_not_distinct",
                "path": operation,
                "detail": "compat/transitional operation must name a distinct canonical_family.",
            }));
        }
        if status == "compat"
            && topology_budget
                .get("compat_operations")
                .and_then(|value| value.get("allowed"))
                .and_then(|value| value.as_bool())
                != Some(true)
        {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_topology_compat_budget_disabled",
                "path": operation,
                "detail": "compat operation present but compat budget is disabled.",
            }));
        }
    }
    let canonical_pairs: std::collections::BTreeSet<(String, String)> = topo_ops
        .values()
        .filter(|contract| contract.get("status").and_then(|value| value.as_str()) == Some("canonical"))
        .map(|contract| {
            (
                contract
                    .get("canonical_family")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
                contract
                    .get("mutation_level")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
            )
        })
        .collect();
    for (operation, contract) in topo_ops {
        if contract.get("status").and_then(|value| value.as_str()) != Some("transitional") {
            continue;
        }
        let key = (
            contract
                .get("canonical_family")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            contract
                .get("mutation_level")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
        );
        if !canonical_pairs.contains(&key) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_topology_transitional_without_canonical",
                "path": operation,
                "detail": "transitional operation has no canonical operation in the same family and mutation level.",
            }));
        }
    }
    findings
}
