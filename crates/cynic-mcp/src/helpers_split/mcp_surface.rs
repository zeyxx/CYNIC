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

fn cynic_coord_exposed_tools(exposure: &serde_json::Value) -> Vec<String> {
    exposure
        .get("mcp_servers")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .find(|server| server.get("name").and_then(|value| value.as_str()) == Some("cynic-coord"))
        .and_then(|server| server.get("tools"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|tool| tool.get("name").and_then(|value| value.as_str()))
        .map(ToString::to_string)
        .collect()
}

fn mcp_domain_surface(exposure: &serde_json::Value) -> Vec<serde_json::Value> {
    let Some(domains) = exposure.get("mcp_tool_domains").and_then(|value| value.as_object()) else {
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
    let exposed_tools = cynic_coord_exposed_tools(exposure);
    let exposed_set: std::collections::BTreeSet<String> = exposed_tools.iter().cloned().collect();
    let mut domain_tools = std::collections::BTreeSet::new();
    let mut findings = Vec::new();

    let Some(domains) = exposure.get("mcp_tool_domains").and_then(|value| value.as_object()) else {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "mcp_tool_domains_missing",
            "path": "infra/agent-exposure.json",
            "detail": "agent exposure SSOT must classify cynic-coord MCP tools by domain"
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
    let exposed_tools = cynic_coord_exposed_tools(&exposure);
    let domains = mcp_domain_surface(&exposure);
    let findings = mcp_surface_findings(&exposure);
    let exposed_tool_count = exposed_tools.len();
    let status = if findings.is_empty() { "ok" } else { "violation" };
    Ok(serde_json::json!({
        "schema": "cynic.mcp_surface_manifest.v1",
        "server": "cynic-coord",
        "canonical_binary": "crates/cynic-mcp",
        "exposure_ssot": "infra/agent-exposure.json",
        "fallback": {
            "python_rest_mcp": ".cortex/mcp/python_rest_mcp.py",
            "role": "minimal_coordination_tasks_lifecycle_status"
        },
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
