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

fn mcp_protocol_capabilities(exposure: &serde_json::Value) -> Vec<String> {
    crate::protocol::source_capabilities(exposure)
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
                "operator_tools": payload.get("operator_tools").cloned().unwrap_or_else(|| serde_json::json!([])),
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
    let exposed_tools = crate::protocol::exposed_tool_names(exposure);
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

fn agent_constitution_status() -> cynic_protocol::AgentConstitutionStatus {
    cynic_protocol::reconcile_agent_execution_capabilities(
        &cynic_protocol::AgentExecutionRuntimeObservation::unobserved(),
    )
}

fn mcp_surface_manifest(
    project_root: &str,
    session_identity: &crate::session_identity::SessionIdentity,
    request_meta_observed: bool,
) -> Result<serde_json::Value, McpError> {
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
    let desired_contract_digest = cynic_protocol::digest_json(&exposure).map_err(|error| {
        McpError::internal_error(
            format!("cannot digest desired agent exposure: {error}"),
            None,
        )
    })?;
    let compiled_contract_digest =
        cynic_protocol::digest_json(&compiled_exposure).map_err(|error| {
            McpError::internal_error(
                format!("cannot digest compiled agent exposure: {error}"),
                None,
            )
        })?;
    let desired_tools = crate::protocol::exposed_tool_names(&exposure);
    let compiled_capabilities = crate::protocol::exposed_tool_names(&compiled_exposure);
    let mut compatibility_capabilities = compiled_capabilities.clone();
    compatibility_capabilities.extend(mcp_protocol_capabilities(&compiled_exposure));
    let exposed_tools = compiled_capabilities.clone();
    let domains = mcp_domain_surface(&compiled_exposure);
    let mut findings = mcp_surface_findings(&exposure);
    findings.extend(mcp_surface_findings(&compiled_exposure));
    if desired_contract_digest != compiled_contract_digest {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "runtime_contract_projection_mismatch",
            "desired_contract_digest": desired_contract_digest,
            "compiled_contract_digest": compiled_contract_digest,
            "detail": "the running artifact was not compiled from the currently desired agent exposure contract"
        }));
    }
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
    let compiled_command = compiled_exposure
        .pointer("/mcp_servers/0/command")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if observed_binary != Some(compiled_command) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "runtime_binary_identity_mismatch",
            "compiled_command": compiled_command,
            "observed_executable": observed_executable.clone(),
        }));
    }
    let desired_handshake = exposure
        .get("session_handshake")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let requirement = cynic_protocol::RuntimeContractRequirement {
        contract_id: desired_handshake
            .get("contract_id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        contract_revision: desired_handshake
            .get("contract_revision")
            .and_then(|value| value.as_u64())
            .unwrap_or_default(),
        required_capabilities: desired_handshake
            .get("required_capabilities")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str())
            .map(ToString::to_string)
            .collect(),
        contract_digest: Some(desired_contract_digest.clone()),
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
            capabilities: Some(compatibility_capabilities),
            contract_digest: Some(compiled_contract_digest.clone()),
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
    let transport = if std::env::var("CYNIC_MCP_TRANSPORT")
        .ok()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("streamable_http"))
    {
        crate::transport::STREAMABLE_HTTP_TRANSPORT_LABEL
    } else {
        "stdio_legacy"
    };
    let protocol_discovery =
        crate::protocol::discovery_result(&compiled_exposure, request_meta_observed, transport);
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
        "compiled_command": compiled_command,
        "observed_executable": observed_executable,
        "session_handshake": compiled_handshake,
        "protocol_profile": compiled_exposure.get("mcp_protocol_profile").cloned().unwrap_or(serde_json::Value::Null),
        "server_discovery": protocol_discovery,
        "runtime_compatibility": compatibility,
        "session_identity": session_identity,
        "constitutional_execution": agent_constitution_status(),
        "exposure_ssot": "infra/agent-exposure.json",
        "compiled_contract_digest": compiled_contract_digest,
        "desired_projection": {
            "contract_digest": desired_contract_digest,
            "session_handshake": desired_handshake,
            "exposed_tool_count": desired_tools.len(),
            "exposed_tools": desired_tools
        },
        "agent_shell_fallback": serde_json::Value::Null,
        "policy": {
            "agent_primary_surface": "mcp",
            "operator_primary_surface": "cynic",
            "make_surface": "validation_dev_only",
            "entrypoints": "role_separated_no_agent_shell_fallback"
        },
        "domains": domains,
        "exposed_tool_count": exposed_tool_count,
        "exposed_tools": exposed_tools,
        "findings": findings,
        "status": status,
    }))
}
