fn infra_consumption_contract_path(project_root: &str) -> PathBuf {
    PathBuf::from(project_root).join("axioms/infra-consumption-v1.json")
}

fn validate_infra_consumption_contract(project_root: &str) -> Vec<serde_json::Value> {
    let path = infra_consumption_contract_path(project_root);
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(e) => {
            return vec![serde_json::json!({
                "severity": "critical",
                "kind": "infra_consumption_contract_missing",
                "path": path.display().to_string(),
                "detail": format!("cannot read infra consumption contract: {e}"),
            })];
        }
    };

    let contract: serde_json::Value = match serde_json::from_str(&content) {
        Ok(contract) => contract,
        Err(e) => {
            return vec![serde_json::json!({
                "severity": "critical",
                "kind": "infra_consumption_contract_invalid_json",
                "path": path.display().to_string(),
                "detail": format!("invalid JSON: {e}"),
            })];
        }
    };

    infra_consumption_contract_findings(project_root, &path.display().to_string(), &contract)
}

fn infra_consumption_contract_findings(
    project_root: &str,
    path: &str,
    contract: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();

    if contract.get("schema").and_then(|value| value.as_str()) != Some("cynic.infra_consumption.v1")
    {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "unexpected_infra_consumption_schema",
            "path": path,
            "detail": format!(
                "expected cynic.infra_consumption.v1, got {:?}",
                contract.get("schema").and_then(|value| value.as_str())
            ),
        }));
    }
    if contract.get("status").and_then(|value| value.as_str()) != Some("normative") {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "infra_consumption_not_normative",
            "path": path,
            "detail": "infra consumption must be normative before it gates organism lifecycle surfaces.",
        }));
    }

    for field in [
        "capability_roles",
        "required_ports",
        "native_lifecycle",
        "service_placement",
        "scan_scope",
    ] {
        let present = match contract.get(field) {
            Some(serde_json::Value::Array(items)) => !items.is_empty(),
            Some(serde_json::Value::Object(items)) => !items.is_empty(),
            Some(serde_json::Value::String(text)) => !text.is_empty(),
            Some(serde_json::Value::Null) | None => false,
            Some(_) => true,
        };
        if !present {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "infra_consumption_required_field_missing",
                "path": path,
                "detail": format!("infra consumption contract must declare {field}."),
            }));
        }
    }

    if contract
        .get("service_placement")
        .and_then(|value| value.get("surrealdb"))
        .and_then(|value| value.get("role"))
        .and_then(|value| value.as_str())
        != Some("storage_primary")
    {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "surrealdb_not_bound_to_storage_primary",
            "path": path,
            "detail": "SurrealDB must be consumed through the storage_primary role.",
        }));
    }

    let compiled_patterns = infra_consumption_compiled_patterns(contract, path, &mut findings);
    let scoped_files = infra_consumption_scoped_files(project_root, contract);

    for forbidden in contract
        .get("native_lifecycle")
        .and_then(|value| value.get("forbidden_on_cortex"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
    {
        if !compiled_patterns
            .iter()
            .any(|(_, _, _, regex)| regex.is_match(forbidden))
        {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "infra_consumption_forbidden_on_cortex_uncovered",
                "path": path,
                "detail": format!(
                    "native_lifecycle.forbidden_on_cortex entry is not covered by forbidden_patterns: {forbidden}"
                ),
            }));
        }
    }

    for (rule_id, severity, detail, regex) in compiled_patterns {
        for scoped_path in &scoped_files {
            let Ok(text) = fs::read_to_string(scoped_path) else {
                continue;
            };
            for (index, line) in text.lines().enumerate() {
                if infra_consumption_is_allowed_literal(line, contract) {
                    continue;
                }
                if regex.is_match(line) {
                    findings.push(serde_json::json!({
                        "severity": severity,
                        "kind": format!("infra_consumption_forbidden_{rule_id}"),
                        "path": format!("{}:{}", relative_project_path(project_root, scoped_path), index + 1),
                        "detail": detail,
                    }));
                }
            }
        }
    }

    findings
}

fn infra_consumption_compiled_patterns(
    contract: &serde_json::Value,
    path: &str,
    findings: &mut Vec<serde_json::Value>,
) -> Vec<(String, String, String, regex::Regex)> {
    let mut compiled = Vec::new();
    for rule in contract
        .get("forbidden_patterns")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
    {
        let Some(rule_id) = rule.get("id").and_then(|value| value.as_str()) else {
            continue;
        };
        let Some(pattern) = rule.get("pattern").and_then(|value| value.as_str()) else {
            continue;
        };
        match regex::Regex::new(pattern) {
            Ok(regex) => compiled.push((
                rule_id.to_string(),
                rule.get("severity")
                    .and_then(|value| value.as_str())
                    .unwrap_or("critical")
                    .to_string(),
                rule.get("detail")
                    .and_then(|value| value.as_str())
                    .unwrap_or("forbidden infrastructure consumption pattern.")
                    .to_string(),
                regex,
            )),
            Err(error) => findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "infra_consumption_invalid_forbidden_pattern",
                "path": path,
                "detail": format!("invalid forbidden pattern {rule_id}: {error}"),
            })),
        }
    }
    compiled
}

fn infra_consumption_scoped_files(project_root: &str, contract: &serde_json::Value) -> Vec<PathBuf> {
    let mut paths = std::collections::BTreeSet::new();
    let root = PathBuf::from(project_root);
    for pattern in contract
        .get("scan_scope")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
    {
        let glob_pattern = root.join(pattern).display().to_string();
        let Ok(entries) = glob::glob(&glob_pattern) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.is_file() {
                paths.insert(entry);
            }
        }
    }
    paths.into_iter().collect()
}

fn infra_consumption_is_allowed_literal(line: &str, contract: &serde_json::Value) -> bool {
    contract
        .get("allowed_literals")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .any(|literal| line.contains(literal))
}

fn relative_project_path(project_root: &str, path: &std::path::Path) -> String {
    let root = std::path::Path::new(project_root);
    match path.strip_prefix(root) {
        Ok(relative) => relative.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}
