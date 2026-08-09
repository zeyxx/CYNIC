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
