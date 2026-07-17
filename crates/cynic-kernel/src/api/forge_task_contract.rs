use cynic_host::runtime_registry::resolve_runtime_registry_path;

pub(crate) fn validate_repo_affecting_hermes_task(
    kind: &str,
    domain: &str,
    content: &str,
) -> Result<(), String> {
    if kind != "hermes" || !matches!(domain, "organ-forge" | "repo-lifecycle") {
        return Ok(());
    }
    let parsed: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| format!("agent task content must be valid JSON for {domain}: {e}"))?;
    let has_declared_targets = ["coord_targets", "targets", "repo_targets", "files"]
        .iter()
        .any(|key| {
            parsed
                .get(key)
                .and_then(|value| value.as_array())
                .map(|items| !items.is_empty())
                .unwrap_or(false)
        });
    if !has_declared_targets {
        return Err(
            "repo-affecting hermes tasks must declare coord_targets, targets, repo_targets, or files"
                .to_string(),
        );
    }
    validate_forge_lifecycle_contract(&parsed, load_registry().as_ref())
}

fn validate_forge_lifecycle_contract(
    parsed: &serde_json::Value,
    registry: Option<&serde_json::Value>,
) -> Result<(), String> {
    let lifecycle_name = required_non_empty_string(parsed, "lifecycle")?;
    let contract_fields = [
        "orchestrator_organ",
        "build_runner_role",
        "deploy_target_role",
        "build_target",
        "deploy_target",
    ];
    for field in contract_fields {
        required_non_empty_string(parsed, field)?;
    }

    let Some(registry) = registry else {
        return Ok(());
    };
    let Some(lifecycle) = registry
        .get("lifecycle")
        .and_then(|value| value.get(lifecycle_name))
    else {
        return Ok(());
    };

    require_registry_match(parsed, lifecycle, "orchestrator_organ")?;
    require_registry_match(parsed, lifecycle, "build_runner_role")?;
    require_registry_match(parsed, lifecycle, "deploy_target_role")?;
    require_registry_json_match(parsed, lifecycle, "runtime_paths")?;
    require_registry_json_match(parsed, lifecycle, "runtime_path_proofs")?;
    require_registry_json_match(parsed, lifecycle, "activation_evidence_contract")?;

    let nodes = registry
        .get("nodes")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let build_role = lifecycle
        .get("build_runner_role")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let deploy_role = lifecycle
        .get("deploy_target_role")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let deploy_flag = lifecycle
        .get("deploy_target_flag")
        .and_then(|value| value.as_str())
        .unwrap_or("deploy_target");
    if let Some(expected_build_target) = node_name_for_role(&nodes, build_role, None) {
        require_content_value(parsed, "build_target", &expected_build_target)?;
    }
    if let Some(expected_deploy_target) = node_name_for_role(&nodes, deploy_role, Some(deploy_flag))
    {
        require_content_value(parsed, "deploy_target", &expected_deploy_target)?;
    }
    Ok(())
}

fn required_non_empty_string<'a>(
    parsed: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, String> {
    let value = parsed
        .get(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if value.is_empty() {
        return Err(format!("repo-affecting forge tasks must declare {field}"));
    }
    Ok(value)
}

fn require_registry_match(
    parsed: &serde_json::Value,
    lifecycle: &serde_json::Value,
    field: &str,
) -> Result<(), String> {
    if let Some(expected) = lifecycle.get(field).and_then(|value| value.as_str()) {
        require_content_value(parsed, field, expected)?;
    }
    Ok(())
}

fn require_registry_json_match(
    parsed: &serde_json::Value,
    lifecycle: &serde_json::Value,
    field: &str,
) -> Result<(), String> {
    if let Some(expected) = lifecycle.get(field)
        && parsed.get(field) != Some(expected)
    {
        return Err(format!(
            "repo-affecting forge tasks must match registry {field}"
        ));
    }
    Ok(())
}

fn require_content_value(
    parsed: &serde_json::Value,
    field: &str,
    expected: &str,
) -> Result<(), String> {
    if parsed.get(field).and_then(|value| value.as_str()) != Some(expected) {
        return Err(format!(
            "repo-affecting forge tasks must match registry {field}={expected}"
        ));
    }
    Ok(())
}

fn node_name_for_role(
    nodes: &[serde_json::Value],
    role: &str,
    deploy_flag: Option<&str>,
) -> Option<String> {
    nodes
        .iter()
        .find(|node| {
            node.get("role").and_then(|value| value.as_str()) == Some(role)
                && deploy_flag
                    .map(|flag| node.get(flag).and_then(|value| value.as_bool()) == Some(true))
                    .unwrap_or(true)
        })
        .and_then(|node| node.get("name").and_then(|value| value.as_str()))
        .map(String::from)
}

fn load_registry() -> Option<serde_json::Value> {
    let registry_path = match resolve_runtime_registry_path(None) {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!(error = %error, "failed to resolve runtime registry");
            return None;
        }
    };
    let content = match std::fs::read_to_string(&registry_path) {
        Ok(content) => content,
        Err(read_error) => {
            tracing::warn!(error = %read_error, path = %registry_path.display(), "failed to read registry.json");
            return None;
        }
    };
    match serde_json::from_str(&content) {
        Ok(registry) => Some(registry),
        Err(parse_error) => {
            tracing::warn!(error = %parse_error, path = %registry_path.display(), "failed to parse registry.json");
            None
        }
    }
}
