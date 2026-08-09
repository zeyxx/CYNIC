fn api_topology_contract_findings(
    project_root: &str,
    openapi: &serde_json::Value,
    topology: &serde_json::Value,
    alignment: &serde_json::Value,
    discovery_rs: &str,
    middleware_rs: &str,
) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();
    let openapi_path = "docs/api/openapi.json";
    let topology_path = "axioms/api-topology-v1.json";
    let alignment_path = "axioms/api-layer-alignment-v1.json";
    let discovery_path = "crates/cynic-kernel/src/api/rest/discovery.rs";
    let middleware_path = "crates/cynic-kernel/src/api/rest/middleware.rs";

    if topology.get("schema").and_then(|value| value.as_str()) != Some("cynic.api_topology.v1") {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "unexpected_api_topology_schema",
            "path": topology_path,
            "detail": format!(
                "expected cynic.api_topology.v1, got {:?}",
                topology.get("schema").and_then(|value| value.as_str())
            ),
        }));
    }
    if topology.get("status").and_then(|value| value.as_str()) != Some("normative") {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_topology_not_normative",
            "path": topology_path,
            "detail": "api-topology-v1 must be normative before agents can use API geometry.",
        }));
    }

    let operations = openapi_operations_json(openapi);
    let topology_operations: std::collections::BTreeSet<String> = topology
        .get("operations")
        .and_then(|value| value.as_object())
        .map(|items| items.keys().cloned().collect())
        .unwrap_or_default();

    let allowed_layers: std::collections::BTreeSet<String> = topology
        .get("allowed")
        .and_then(|value| value.get("layers"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect();
    let allowed_mutation_levels: std::collections::BTreeSet<String> = topology
        .get("allowed")
        .and_then(|value| value.get("mutation_levels"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect();
    let allowed_statuses: std::collections::BTreeSet<String> = topology
        .get("allowed")
        .and_then(|value| value.get("statuses"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect();
    let topology_budget = topology
        .get("topology_budget")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();

    let expected_extension = serde_json::json!({
        "schema": topology.get("schema"),
        "path": topology_path,
    });
    if openapi.get("x-cynic-api-topology-contract") != Some(&expected_extension) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "openapi_topology_extension_drift",
            "path": openapi_path,
            "detail": "OpenAPI must link api-topology-v1 through x-cynic-api-topology-contract.",
        }));
    }
    let expected_alignment_extension = serde_json::json!({
        "schema": alignment.get("schema"),
        "path": alignment_path,
    });
    if openapi.get("x-cynic-api-layer-alignment-contract") != Some(&expected_alignment_extension) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "openapi_layer_alignment_extension_drift",
            "path": openapi_path,
            "detail": "OpenAPI must link api-layer-alignment-v1 through x-cynic-api-layer-alignment-contract.",
        }));
    }
    if topology
        .get("source_of_truth")
        .and_then(|value| value.get("layer_alignment"))
        .and_then(|value| value.as_str())
        != Some(alignment_path)
    {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_topology_layer_alignment_source_drift",
            "path": topology_path,
            "detail": format!("source_of_truth.layer_alignment must point to {alignment_path}."),
        }));
    }

    let alignment_layers: std::collections::BTreeSet<String> = alignment
        .get("layers")
        .and_then(|value| value.as_object())
        .map(|layers| layers.keys().cloned().collect())
        .unwrap_or_default();
    for layer in allowed_layers.difference(&alignment_layers) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_topology_alignment_layer_missing",
            "path": alignment_path,
            "detail": format!("missing layer alignment for topology layer {layer:?}."),
        }));
    }
    for layer in alignment_layers.difference(&allowed_layers) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_topology_alignment_layer_unknown",
            "path": alignment_path,
            "detail": format!("alignment declares layer {layer:?} absent from topology allowed.layers."),
        }));
    }

    findings.extend(validate_api_public_discovery(
        openapi,
        topology,
        discovery_rs,
        middleware_rs,
        discovery_path,
        middleware_path,
    ));
    findings.extend(validate_api_operation_geometry(
        &operations,
        topology,
        &topology_operations,
        &allowed_layers,
        &allowed_mutation_levels,
        &allowed_statuses,
        &topology_budget,
    ));
    let _ = project_root;
    findings
}

