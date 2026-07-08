fn command_surfaces_contract_path(project_root: &str) -> PathBuf {
    PathBuf::from(project_root).join("axioms/command-surfaces-v1.json")
}

fn validate_command_surfaces_contract(project_root: &str) -> Vec<serde_json::Value> {
    let path = command_surfaces_contract_path(project_root);
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(e) => {
            return vec![serde_json::json!({
                "severity": "critical",
                "kind": "command_surfaces_contract_missing",
                "path": path.display().to_string(),
                "detail": format!("cannot read command surfaces contract: {e}"),
            })];
        }
    };

    let contract: serde_json::Value = match serde_json::from_str(&content) {
        Ok(contract) => contract,
        Err(e) => {
            return vec![serde_json::json!({
                "severity": "critical",
                "kind": "command_surfaces_contract_invalid_json",
                "path": path.display().to_string(),
                "detail": e.to_string(),
            })];
        }
    };

    let exposure = load_agent_exposure(project_root).unwrap_or_else(|_| serde_json::json!({}));
    let make_targets = command_surface_make_targets(project_root);
    command_surface_findings(
        "axioms/command-surfaces-v1.json",
        &contract,
        &exposure,
        &make_targets,
    )
}

fn command_surface_findings(
    path: &str,
    contract: &serde_json::Value,
    exposure: &serde_json::Value,
    make_targets: &std::collections::BTreeSet<String>,
) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();

    if contract.get("schema").and_then(|value| value.as_str()) != Some("cynic.command_surfaces.v1")
    {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "unexpected_command_surfaces_schema",
            "path": path,
            "detail": format!(
                "expected cynic.command_surfaces.v1, got {:?}",
                contract.get("schema").and_then(|value| value.as_str())
            ),
        }));
    }

    let required_roles = [
        "human_operator",
        "interactive_agent",
        "machine_fallback",
        "validation_dev",
        "runtime_daemon",
    ];
    let roles = contract
        .get("surface_roles")
        .and_then(|value| value.as_object());
    let role_names: std::collections::BTreeSet<&str> = roles
        .map(|roles| roles.keys().map(String::as_str).collect())
        .unwrap_or_default();
    if roles.is_none() {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "command_surface_roles_missing",
            "path": path,
            "detail": "surface_roles must define human_operator, interactive_agent, machine_fallback, validation_dev, and runtime_daemon",
        }));
    }
    let missing_roles: Vec<&str> = required_roles
        .iter()
        .copied()
        .filter(|role| !role_names.contains(role))
        .collect();
    if !missing_roles.is_empty() {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "command_surface_required_roles_missing",
            "path": path,
            "detail": format!("missing roles: {}", missing_roles.join(", ")),
        }));
    }

    let Some(commands) = contract.get("commands").and_then(|value| value.as_array()) else {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "command_surface_commands_missing",
            "path": path,
            "detail": "commands must be a list",
        }));
        return findings;
    };

    let exposed_tools = command_surface_exposed_tools(exposure);
    let required_fields = [
        "name",
        "role",
        "domain",
        "canonical",
        "mutates",
        "mutates_mempool",
        "json_output_required",
    ];

    for command in commands {
        let Some(command_obj) = command.as_object() else {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "command_surface_command_invalid",
                "path": path,
                "detail": "every command entry must be an object",
            }));
            continue;
        };
        let name = command_obj
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let missing_fields: Vec<&str> = required_fields
            .iter()
            .copied()
            .filter(|field| !command_obj.contains_key(*field))
            .collect();
        if !missing_fields.is_empty() {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "command_surface_command_metadata_missing",
                "path": if name.is_empty() { path.to_string() } else { name.to_string() },
                "detail": format!("command is missing fields: {}", missing_fields.join(", ")),
            }));
        }

        let role = command_obj
            .get("role")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        if !role.is_empty() && !role_names.contains(role) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "command_surface_unknown_role",
                "path": name,
                "detail": format!("role {:?} is not declared in surface_roles", role),
            }));
        }

        if matches!(role, "interactive_agent" | "machine_fallback")
            && command_obj
                .get("json_output_required")
                .and_then(|value| value.as_bool())
                != Some(true)
        {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "machine_command_without_json_output",
                "path": name,
                "detail": "interactive-agent and machine-fallback surfaces must emit JSON/JSONL",
            }));
        }

        if command_obj
            .get("mutates_mempool")
            .and_then(|value| value.as_bool())
            == Some(true)
            && command_obj
                .get("read_back_required")
                .and_then(|value| value.as_bool())
                != Some(true)
        {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "mempool_mutation_without_read_back",
                "path": name,
                "detail": "commands that mutate the Kernel mempool must require read-back",
            }));
        }

        if role == "interactive_agent" && !exposed_tools.contains(name) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "interactive_agent_command_not_exposed",
                "path": name,
                "detail": "interactive-agent commands must be declared in infra/agent-exposure.json",
            }));
        }

        if let Some(target) = name.strip_prefix("make ")
            && !make_targets.contains(target)
        {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "make_surface_target_missing",
                "path": name,
                "detail": format!("make target {:?} is declared as a surface but not implemented", target),
            }));
        }

        if name.starts_with("make ")
            && command_obj.get("canonical").and_then(|value| value.as_bool()) == Some(true)
            && command_obj.get("domain").and_then(|value| value.as_str()) == Some("lifecycle")
            && command_obj.get("mutates").and_then(|value| value.as_bool()) == Some(true)
        {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "make_lifecycle_mutation_marked_canonical",
                "path": name,
                "detail": "mutating lifecycle make targets must remain fallback/dev wrappers, not canonical control surfaces",
            }));
        }
    }

    findings
}

fn command_surface_exposed_tools(exposure: &serde_json::Value) -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
    if let Some(domains) = exposure.get("mcp_tool_domains").and_then(|value| value.as_object()) {
        for domain in domains.values() {
            if let Some(tools) = domain.get("tools").and_then(|value| value.as_array()) {
                for tool in tools.iter().filter_map(|value| value.as_str()) {
                    names.insert(tool.to_string());
                }
            }
            if let Some(tools) = domain.get("fallback_tools").and_then(|value| value.as_array()) {
                for tool in tools.iter().filter_map(|value| value.as_str()) {
                    names.insert(tool.to_string());
                }
            }
        }
    }
    if let Some(servers) = exposure.get("mcp_servers").and_then(|value| value.as_array()) {
        for server in servers {
            if let Some(tools) = server.get("tools").and_then(|value| value.as_array()) {
                for tool in tools {
                    if let Some(name) = tool.get("name").and_then(|value| value.as_str()) {
                        names.insert(name.to_string());
                    }
                }
            }
        }
    }
    names
}

fn command_surface_make_targets(project_root: &str) -> std::collections::BTreeSet<String> {
    let output = std::process::Command::new("make")
        .arg("-pn")
        .current_dir(project_root)
        .stderr(Stdio::null())
        .output();

    let Ok(output) = output else {
        return std::collections::BTreeSet::new();
    };
    if !output.status.success() {
        return std::collections::BTreeSet::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            if !line.contains(':') || line.starts_with(['\t', '.', '#']) {
                return None;
            }
            let target = line.split(':').next()?.trim();
            if target.is_empty() || target.contains(' ') || target.starts_with('%') {
                return None;
            }
            Some(target.to_string())
        })
        .collect()
}
