fn agent_exposure_path(project_root: &str) -> PathBuf {
    PathBuf::from(project_root).join("infra/agent-exposure.json")
}

fn load_agent_exposure(project_root: &str) -> Result<serde_json::Value, McpError> {
    let path = agent_exposure_path(project_root);
    let content = fs::read_to_string(&path).map_err(|e| {
        McpError::internal_error(
            format!("cannot read agent exposure SSOT {}: {e}", path.display()),
            None,
        )
    })?;
    serde_json::from_str(&content).map_err(|e| {
        McpError::internal_error(
            format!("cannot parse agent exposure SSOT {}: {e}", path.display()),
            None,
        )
    })
}

fn active_mcp_exposed_tools(exposure: &serde_json::Value) -> Vec<String> {
    exposure
        .get("mcp_servers")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .find(|server| server.get("name").and_then(|value| value.as_str()) == Some("cynic"))
        .and_then(|server| server.get("tools"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|tool| tool.get("name").and_then(|value| value.as_str()))
        .map(ToString::to_string)
        .collect()
}

fn mcp_domain_surface(exposure: &serde_json::Value) -> Vec<serde_json::Value> {
    let Some(domains) = exposure
        .get("mcp_tool_domains")
        .and_then(|value| value.as_object())
    else {
        return Vec::new();
    };

    let mut result: Vec<serde_json::Value> = domains
        .iter()
        .map(|(name, payload)| {
            serde_json::json!({
                "name": name,
                "description": payload.get("description").cloned().unwrap_or_else(|| serde_json::json!("")),
                "tools": payload.get("tools").cloned().unwrap_or_else(|| serde_json::json!([])),
                "fallback_tools": payload.get("fallback_tools").cloned().unwrap_or_else(|| serde_json::json!([])),
            })
        })
        .collect();
    result.sort_by(|left, right| {
        left.get("name")
            .and_then(|value| value.as_str())
            .cmp(&right.get("name").and_then(|value| value.as_str()))
    });
    result
}

fn mcp_surface_findings(exposure: &serde_json::Value) -> Vec<serde_json::Value> {
    let exposed_tools = active_mcp_exposed_tools(exposure);
    let exposed_set: std::collections::BTreeSet<String> = exposed_tools.iter().cloned().collect();
    let mut domain_tools = std::collections::BTreeSet::new();
    let mut findings = Vec::new();
    if exposure
        .pointer("/constitutional_execution/execution_contract")
        .and_then(|value| value.as_str())
        != Some("cynic.agent_workload_envelope.v1")
    {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "agent_constitution_contract_missing",
            "path": "infra/agent-exposure.json",
        }));
    }

    let Some(domains) = exposure
        .get("mcp_tool_domains")
        .and_then(|value| value.as_object())
    else {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "mcp_tool_domains_missing",
            "path": "infra/agent-exposure.json",
            "detail": "agent exposure SSOT must classify active cynic MCP tools by domain"
        }));
        return findings;
    };

    for (domain, payload) in domains {
        let Some(tools) = payload.get("tools").and_then(|value| value.as_array()) else {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "mcp_domain_without_tools",
                "domain": domain,
            }));
            continue;
        };
        for tool in tools.iter().filter_map(|value| value.as_str()) {
            domain_tools.insert(tool.to_string());
        }
    }

    for tool in exposed_set.difference(&domain_tools) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "exposed_tool_missing_domain",
            "tool": tool,
        }));
    }
    for tool in domain_tools.difference(&exposed_set) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "domain_tool_not_exposed",
            "tool": tool,
        }));
    }
    findings
}

fn mcp_surface_manifest(project_root: &str) -> Result<serde_json::Value, McpError> {
    let exposure = load_agent_exposure(project_root)?;
    let compiled_exposure: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../infra/agent-exposure.json"
    ))
    .map_err(|error| {
        McpError::internal_error(
            format!("cannot parse compiled agent exposure contract: {error}"),
            None,
        )
    })?;
    let exposed_tools = active_mcp_exposed_tools(&exposure);
    let compiled_capabilities = active_mcp_exposed_tools(&compiled_exposure);
    let domains = mcp_domain_surface(&exposure);
    let mut findings = mcp_surface_findings(&exposure);
    let desired_command = exposure
        .pointer("/mcp_servers/0/command")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let observed_executable = std::env::current_exe()
        .ok()
        .map(|path| path.display().to_string());
    let observed_binary = observed_executable
        .as_deref()
        .and_then(|path| std::path::Path::new(path).file_name())
        .and_then(|name| name.to_str());
    if observed_binary != Some(desired_command) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "runtime_binary_identity_mismatch",
            "desired_command": desired_command,
            "observed_executable": observed_executable.clone(),
        }));
    }
    let handshake = exposure
        .get("session_handshake")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let requirement = cynic_protocol::RuntimeContractRequirement {
        contract_id: handshake
            .get("contract_id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        contract_revision: handshake
            .get("contract_revision")
            .and_then(|value| value.as_u64())
            .unwrap_or_default(),
        required_capabilities: handshake
            .get("required_capabilities")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str())
            .map(ToString::to_string)
            .collect(),
    };
    let compiled_handshake = compiled_exposure
        .get("session_handshake")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let compatibility = cynic_protocol::reconcile_runtime_compatibility(
        &requirement,
        &cynic_protocol::RuntimeContractObservation {
            component: "cynic-mcp".to_string(),
            artifact_version: Some(env!("CYNIC_VERSION").to_string()),
            contract_id: compiled_handshake
                .get("contract_id")
                .and_then(|value| value.as_str())
                .map(ToString::to_string),
            contract_revision: compiled_handshake
                .get("contract_revision")
                .and_then(|value| value.as_u64()),
            capabilities: Some(compiled_capabilities),
            evidence_refs: vec![format!("artifact:cynic-mcp:{}", env!("CYNIC_VERSION"))],
            unknowns: Vec::new(),
        },
    );
    if compatibility.status != cynic_protocol::RuntimeCompatibilityStatus::Compatible {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "runtime_contract_incompatible",
            "compatibility": compatibility,
        }));
    }
    let exposed_tool_count = exposed_tools.len();
    let status = if findings.is_empty() {
        "observed"
    } else {
        "violation"
    };
    Ok(serde_json::json!({
        "schema": "cynic.mcp_surface_manifest.v1",
        "server": "cynic",
        "canonical_binary": "crates/cynic-mcp",
        "desired_command": desired_command,
        "observed_executable": observed_executable,
        "session_handshake": handshake,
        "runtime_compatibility": compatibility,
        "constitutional_execution": exposure.get("constitutional_execution").cloned().unwrap_or(serde_json::Value::Null),
        "exposure_ssot": "infra/agent-exposure.json",
        "fallback": serde_json::Value::Null,
        "policy": {
            "agent_primary_surface": "mcp",
            "operator_primary_surface": "cynic",
            "make_surface": "validation_dev_only",
            "entrypoints": "internal_or_fallback_transport"
        },
        "domains": domains,
        "exposed_tool_count": exposed_tool_count,
        "exposed_tools": exposed_tools,
        "findings": findings,
        "status": status,
    }))
}
