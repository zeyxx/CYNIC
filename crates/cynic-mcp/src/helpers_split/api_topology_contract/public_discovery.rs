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

