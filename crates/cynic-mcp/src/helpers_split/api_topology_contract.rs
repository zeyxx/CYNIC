fn api_topology_contract_paths(project_root: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf, PathBuf) {
    let root = PathBuf::from(project_root);
    (
        root.join("docs/api/openapi.json"),
        root.join("axioms/api-topology-v1.json"),
        root.join("axioms/api-layer-alignment-v1.json"),
        root.join("crates/cynic-kernel/src/api/rest/discovery.rs"),
        root.join("crates/cynic-kernel/src/api/rest/middleware.rs"),
    )
}

fn validate_api_topology_contract(project_root: &str) -> Vec<serde_json::Value> {
    let (openapi_path, topology_path, alignment_path, discovery_path, middleware_path) =
        api_topology_contract_paths(project_root);
    let mut findings = Vec::new();

    let openapi = match read_json_contract(&openapi_path, "openapi_contract", &mut findings) {
        Some(value) => value,
        None => return findings,
    };
    let topology = match read_json_contract(&topology_path, "api_topology_contract", &mut findings) {
        Some(value) => value,
        None => return findings,
    };
    let alignment =
        match read_json_contract(&alignment_path, "api_layer_alignment_contract", &mut findings) {
            Some(value) => value,
            None => return findings,
        };
    let discovery_rs = match fs::read_to_string(&discovery_path) {
        Ok(text) => text,
        Err(_) => {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_discovery_rs_missing",
                "path": relative_project_path(project_root, &discovery_path),
                "detail": format!(
                    "{} must exist for API discovery validation.",
                    relative_project_path(project_root, &discovery_path)
                ),
            }));
            return findings;
        }
    };
    let middleware_rs = match fs::read_to_string(&middleware_path) {
        Ok(text) => text,
        Err(_) => {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_middleware_rs_missing",
                "path": relative_project_path(project_root, &middleware_path),
                "detail": format!(
                    "{} must exist for API discovery validation.",
                    relative_project_path(project_root, &middleware_path)
                ),
            }));
            return findings;
        }
    };

    findings.extend(api_topology_contract_findings(
        project_root,
        &openapi,
        &topology,
        &alignment,
        &discovery_rs,
        &middleware_rs,
    ));
    findings
}

fn read_json_contract(
    path: &PathBuf,
    kind: &str,
    findings: &mut Vec<serde_json::Value>,
) -> Option<serde_json::Value> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": format!("{kind}_missing"),
                "path": path.display().to_string(),
                "detail": format!("{} must exist for API topology validation.", path.display()),
            }));
            return None;
        }
    };
    match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(serde_json::Value::Object(_)) => serde_json::from_str(&content).ok(),
        Ok(_) => {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": format!("{kind}_invalid_shape"),
                "path": path.display().to_string(),
                "detail": "contract must be a JSON object.",
            }));
            None
        }
        Err(error) => {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": format!("{kind}_invalid_json"),
                "path": path.display().to_string(),
                "detail": error.to_string(),
            }));
            None
        }
    }
}

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

fn openapi_operations_json(openapi: &serde_json::Value) -> std::collections::BTreeSet<String> {
    let mut operations = std::collections::BTreeSet::new();
    if let Some(paths) = openapi.get("paths").and_then(|value| value.as_object()) {
        for (path, methods) in paths {
            if let Some(methods) = methods.as_object() {
                for method in methods.keys() {
                    if !method.starts_with("x-") {
                        operations.insert(format!("{} {}", method.to_uppercase(), path));
                    }
                }
            }
        }
    }
    operations
}

fn api_operation_root(operation: &str) -> String {
    let path = operation.split_once(' ').map(|(_, path)| path).unwrap_or(operation);
    format!("/{}", path.trim_start_matches('/').split('/').next().unwrap_or_default())
}

fn validate_api_public_discovery(
    openapi: &serde_json::Value,
    topology: &serde_json::Value,
    discovery_rs: &str,
    middleware_rs: &str,
    discovery_path: &str,
    middleware_path: &str,
) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();
    let required_discovery = [
        (
            "GET /openapi.json",
            "docs/api/openapi.json",
            "OPENAPI_JSON",
            "openapi_handler",
        ),
        (
            "GET /api/topology",
            "axioms/api-topology-v1.json",
            "API_TOPOLOGY_JSON",
            "api_topology_handler",
        ),
        (
            "GET /api/layer-alignment",
            "axioms/api-layer-alignment-v1.json",
            "API_LAYER_ALIGNMENT_JSON",
            "api_layer_alignment_handler",
        ),
    ];
    for (operation, served_path, constant_name, handler_name) in required_discovery {
        if topology
            .get("public_discovery_endpoints")
            .and_then(|value| value.get(operation))
            .and_then(|value| value.get("serves"))
            .and_then(|value| value.as_str())
            != Some(served_path)
        {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_public_discovery_serves_drift",
                "path": operation,
                "detail": format!("public_discovery_endpoints must serve {served_path}."),
            }));
        }
        let (_, path) = operation.split_once(' ').unwrap_or(("GET", operation));
        let include_tail = format!("include_str!(\"../../../../../{}\")", served_path);
        if !discovery_rs.contains(&format!("const {constant_name}: &str"))
            || !discovery_rs.contains(&include_tail)
        {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_public_discovery_include_missing",
                "path": discovery_path,
                "detail": format!("{operation} must include {served_path} through {constant_name}."),
            }));
        }
        if !discovery_rs.contains(&format!("pub async fn {handler_name}() -> Response"))
            || !discovery_rs.contains(&format!("json_contract({constant_name})"))
        {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_public_discovery_handler_missing",
                "path": discovery_path,
                "detail": format!("{operation} handler {handler_name} must serve {constant_name}."),
            }));
        }
        if !middleware_rs.contains(&format!("path == \"{path}\"")) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_public_discovery_auth_exemption_missing",
                "path": middleware_path,
                "detail": format!("auth middleware must exempt public discovery path {path}."),
            }));
        }
        let method = operation.split_once(' ').map(|(method, _)| method.to_lowercase()).unwrap_or_default();
        let openapi_operation = openapi
            .get("paths")
            .and_then(|value| value.get(path))
            .and_then(|value| value.get(&method));
        if openapi_operation.is_none() {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_public_discovery_openapi_operation_missing",
                "path": "docs/api/openapi.json",
                "detail": format!("{operation} is missing from OpenAPI."),
            }));
        } else if openapi_operation.and_then(|value| value.get("security")) != Some(&serde_json::json!([])) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_public_discovery_security_not_empty",
                "path": "docs/api/openapi.json",
                "detail": format!("{operation} must declare security=[]."),
            }));
        }
        let discovery_contract = topology
            .get("operations")
            .and_then(|value| value.get(operation));
        if discovery_contract.is_none() {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_public_discovery_topology_operation_missing",
                "path": "axioms/api-topology-v1.json",
                "detail": format!("{operation} is missing from API topology."),
            }));
        } else {
            for (field, expected) in [
                ("layer", serde_json::json!("perception")),
                ("mutation_level", serde_json::json!("read")),
                ("agent_safe_default", serde_json::json!(true)),
                ("canonical_family", serde_json::json!("/api")),
                ("status", serde_json::json!("canonical")),
            ] {
                if discovery_contract.and_then(|value| value.get(field)) != Some(&expected) {
                    findings.push(serde_json::json!({
                        "severity": "critical",
                        "kind": "api_public_discovery_contract_drift",
                        "path": operation,
                        "detail": format!("public discovery {field} must be {expected:?}."),
                    }));
                }
            }
        }
    }
    findings
}

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
