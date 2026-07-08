fn infrastructure_mcp_contract_path(project_root: &str) -> PathBuf {
    PathBuf::from(project_root).join("axioms/infrastructure-mcp-v1.json")
}

fn validate_infrastructure_mcp_contract(project_root: &str) -> Vec<serde_json::Value> {
    let path = infrastructure_mcp_contract_path(project_root);
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(e) => {
            return vec![serde_json::json!({
                "severity": "critical",
                "kind": "infrastructure_mcp_contract_missing",
                "path": path.display().to_string(),
                "detail": format!("cannot read infrastructure MCP contract: {e}"),
            })];
        }
    };

    let contract: serde_json::Value = match serde_json::from_str(&content) {
        Ok(contract) => contract,
        Err(e) => {
            return vec![serde_json::json!({
                "severity": "critical",
                "kind": "infrastructure_mcp_contract_invalid_json",
                "path": path.display().to_string(),
                "detail": e.to_string(),
            })];
        }
    };

    infrastructure_mcp_contract_findings(&path.display().to_string(), &contract)
}

fn infrastructure_mcp_contract_findings(
    path: &str,
    contract: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();

    if contract.get("schema").and_then(|value| value.as_str()) != Some("cynic.infrastructure_mcp.v1")
    {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "unexpected_infrastructure_mcp_schema",
            "path": path,
            "detail": format!(
                "expected cynic.infrastructure_mcp.v1, got {:?}",
                contract.get("schema").and_then(|value| value.as_str())
            ),
        }));
    }

    if contract.get("status").and_then(|value| value.as_str()) != Some("target_contract") {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "infrastructure_mcp_status_invalid",
            "path": path,
            "detail": "infrastructure MCP contract status must remain target_contract until all mutating ops are implemented",
        }));
    }

    let principles: std::collections::BTreeSet<&str> = contract
        .get("principles")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .collect();

    for principle in [
        "deny_by_default",
        "registry_desired_state_first",
        "synapse_dns_is_the_service_contract",
        "mutating_operations_require_preflight_and_audit",
        "private_keys_never_leave_the_node",
    ] {
        if !principles.contains(principle) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "infrastructure_mcp_principle_missing",
                "path": path,
                "detail": format!("principles must include {principle}"),
            }));
        }
    }

    let operations = infrastructure_mcp_operations(contract, path, &mut findings);
    let expected_tools = [
        ("fleet.node.exec", "fleet_node_exec", true),
        ("lxc_exec_bridge.plan", "lxc_exec_bridge_plan", false),
        ("lxc_exec_bridge.bundle", "lxc_exec_bridge_bundle", false),
        ("lxc_exec_bridge.status", "lxc_exec_bridge_status", false),
        ("network.mesh.status", "network_mesh_status", false),
        ("network.mesh.dns.render", "network_mesh_dns_render", false),
        (
            "network.mesh.dns.activation_plan",
            "network_mesh_dns_activation_plan",
            false,
        ),
        (
            "network.mesh.dns.activation_bundle",
            "network_mesh_dns_activation_bundle",
            false,
        ),
        (
            "network.mesh.dns.activation_status",
            "network_mesh_dns_activation_status",
            false,
        ),
        ("storage.primary.plan", "storage_primary_plan", false),
        ("storage.primary.bundle", "storage_primary_bundle", false),
        ("storage.primary.status", "storage_primary_status", false),
        ("storage.primary.readiness", "storage_primary_readiness", false),
    ];

    for (operation, tool, mutates) in expected_tools {
        let item = operations.get(operation);
        let Some(item) = item else {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "infrastructure_mcp_operation_missing",
                "path": path,
                "detail": format!("operation {operation:?} must be declared"),
            }));
            continue;
        };

        if item.get("mcp_tool").and_then(|value| value.as_str()) != Some(tool) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "infrastructure_mcp_tool_drift",
                "path": path,
                "detail": format!(
                    "{operation}.mcp_tool must be {tool:?}, got {:?}",
                    item.get("mcp_tool").and_then(|value| value.as_str())
                ),
            }));
        }

        if item.get("mutates").and_then(|value| value.as_bool()) != Some(mutates) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "infrastructure_mcp_mutability_drift",
                "path": path,
                "detail": format!("{operation}.mutates must be {mutates}"),
            }));
        }
    }

    if let Some(fleet_exec) = operations.get("fleet.node.exec") {
        if fleet_exec
            .get("lxc_bridge_contract")
            .and_then(|value| value.as_str())
            != Some("axioms/lxc-exec-bridge-v1.json")
        {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "infrastructure_mcp_lxc_bridge_contract_missing",
                "path": path,
                "detail": "fleet.node.exec must point to axioms/lxc-exec-bridge-v1.json",
            }));
        }

        let current_coverage: std::collections::BTreeSet<&str> = fleet_exec
            .get("current_coverage")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str())
            .collect();
        if !current_coverage.contains(
            "lxc_structured_precondition_missing_until_exec_bridge_exists_or_transport_configured",
        ) {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "infrastructure_mcp_lxc_precondition_guard_missing",
                "path": path,
                "detail": "fleet.node.exec current_coverage must retain the structured LXC precondition guard",
            }));
        }
    }

    if let Some(readiness) = operations.get("storage.primary.readiness") {
        let composes: std::collections::BTreeSet<&str> = readiness
            .get("composes")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str())
            .collect();
        for dependency in [
            "storage.primary.status",
            "lxc_exec_bridge.status",
            "network.mesh.status",
            "network.mesh.dns.activation_status",
            "fleet.node.status",
            "service.health",
        ] {
            if !composes.contains(dependency) {
                findings.push(serde_json::json!({
                    "severity": "critical",
                    "kind": "infrastructure_mcp_readiness_dependency_missing",
                    "path": path,
                    "detail": format!("storage.primary.readiness must compose {dependency}"),
                }));
            }
        }
    }

    if let Some(storage_status) = operations.get("storage.primary.status") {
        let checks: std::collections::BTreeSet<&str> = storage_status
            .get("preconditions_reported")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str())
            .collect();
        for check in [
            "registry_declares_storage_node",
            "endpoint_is_synapse_dns",
            "bridge_active",
            "fleet_observes_node_running",
            "mesh_reaches_storage",
            "bundle_complete",
            "service_health_healthy",
        ] {
            if !checks.contains(check) {
                findings.push(serde_json::json!({
                    "severity": "critical",
                    "kind": "infrastructure_mcp_storage_status_check_missing",
                    "path": path,
                    "detail": format!("storage.primary.status must report {check}"),
                }));
            }
        }
    }

    for operation in [
        "lxc_exec_bridge.bundle",
        "network.mesh.dns.activation_bundle",
        "storage.primary.bundle",
    ] {
        if let Some(item) = operations.get(operation)
            && item
                .get("writes_local_output_dir")
                .and_then(|value| value.as_bool())
                != Some(true)
        {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "infrastructure_mcp_bundle_output_contract_missing",
                "path": path,
                "detail": format!("{operation} must declare writes_local_output_dir=true"),
            }));
        }
    }

    findings
}

fn infrastructure_mcp_operations<'a>(
    contract: &'a serde_json::Value,
    path: &str,
    findings: &mut Vec<serde_json::Value>,
) -> std::collections::BTreeMap<String, &'a serde_json::Value> {
    let mut operations = std::collections::BTreeMap::new();
    let Some(families) = contract.get("operation_families").and_then(|value| value.as_object()) else {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "infrastructure_mcp_operation_families_missing",
            "path": path,
            "detail": "operation_families must be an object keyed by family name",
        }));
        return operations;
    };

    for family in families.values() {
        let Some(family_obj) = family.as_object() else {
            continue;
        };

        if let Some(family_operations) = family_obj.get("operations").and_then(|value| value.as_object()) {
            for (key, value) in family_operations {
                operations.insert(key.clone(), value);
            }
        }

        for (key, value) in family_obj {
            if key.contains('.') && value.is_object() {
                operations.insert(key.clone(), value);
            }
        }
    }

    operations
}
