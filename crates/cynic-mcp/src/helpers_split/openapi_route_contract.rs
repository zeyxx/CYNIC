fn openapi_route_contract_paths(project_root: &str) -> (PathBuf, PathBuf) {
    let root = PathBuf::from(project_root);
    (
        root.join("crates/cynic-kernel/src/api/rest/mod.rs"),
        root.join("docs/api/openapi.json"),
    )
}

fn validate_openapi_route_contract(project_root: &str) -> Vec<serde_json::Value> {
    let (router_path, openapi_path) = openapi_route_contract_paths(project_root);

    let router_text = match fs::read_to_string(&router_path) {
        Ok(text) => text,
        Err(_) => {
            return vec![serde_json::json!({
                "severity": "critical",
                "kind": "openapi_router_source_missing",
                "path": relative_project_path(project_root, &router_path),
                "detail": "kernel REST router source must exist before OpenAPI route parity can be validated.",
            })];
        }
    };

    let openapi = match read_json_contract(&openapi_path, "openapi_contract", &mut Vec::new()) {
        Some(value) => value,
        None => {
            let mut findings = Vec::new();
            let _ = read_json_contract(&openapi_path, "openapi_contract", &mut findings);
            return findings;
        }
    };

    openapi_route_contract_findings(project_root, &router_text, &openapi)
}

fn openapi_route_contract_findings(
    project_root: &str,
    router_text: &str,
    openapi: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();
    let route_re = regex::Regex::new(r#"\.route\(\s*"([^"]+)""#)
        .expect("route regex must compile");
    let router_paths: std::collections::BTreeSet<String> = route_re
        .captures_iter(router_text)
        .filter_map(|captures| captures.get(1).map(|capture| capture.as_str().to_string()))
        .collect();
    let openapi_paths: std::collections::BTreeSet<String> = openapi
        .get("paths")
        .and_then(|value| value.as_object())
        .map(|paths| paths.keys().cloned().collect())
        .unwrap_or_default();

    for route in router_paths.difference(&openapi_paths) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "openapi_route_missing",
            "path": "docs/api/openapi.json",
            "detail": format!("OpenAPI is missing registered kernel route {route:?}."),
        }));
    }
    for route in openapi_paths.difference(&router_paths) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "openapi_route_stale",
            "path": "docs/api/openapi.json",
            "detail": format!("OpenAPI declares path {route:?} not registered by the kernel router."),
        }));
    }

    let _ = project_root;
    findings
}
