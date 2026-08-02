#[cfg(test)]
fn storage_primary_plan_args(params: &StoragePrimaryPlanParams) -> Result<Vec<String>, McpError> {
    Ok(vec![
        "storage-primary".to_string(),
        "plan".to_string(),
        "--registry".to_string(),
        repo_relative_arg(&params.registry, "infra/registry.json", "registry")?,
    ])
}

#[cfg(test)]
fn storage_primary_bundle_args(
    params: &StoragePrimaryBundleParams,
) -> Result<Vec<String>, McpError> {
    Ok(vec![
        "storage-primary".to_string(),
        "bundle".to_string(),
        "--registry".to_string(),
        repo_relative_arg(&params.registry, "infra/registry.json", "registry")?,
        "--schema".to_string(),
        repo_relative_arg(
            &params.schema,
            "crates/cynic-kernel/src/storage/schema_bootstrap.sql",
            "schema",
        )?,
        "--output-dir".to_string(),
        tmp_output_dir_arg(
            &params.output_dir,
            "/tmp/cynic-storage-primary",
            "output_dir",
        )?,
    ])
}

fn run_storage_primary_plan(
    params: &StoragePrimaryPlanParams,
) -> Result<serde_json::Value, McpError> {
    let registry = project_root().join(repo_relative_arg(
        &params.registry,
        "infra/registry.json",
        "registry",
    )?);
    let schema = project_root().join("crates/cynic-kernel/src/storage/schema_bootstrap.sql");
    serde_json::to_value(cynic_host::storage_primary::build_plan(&registry, &schema))
        .map_err(|error| McpError::internal_error(error.to_string(), None))
}

fn run_storage_primary_bundle(
    params: &StoragePrimaryBundleParams,
) -> Result<serde_json::Value, McpError> {
    let registry = project_root().join(repo_relative_arg(
        &params.registry,
        "infra/registry.json",
        "registry",
    )?);
    let schema = project_root().join(repo_relative_arg(
        &params.schema,
        "crates/cynic-kernel/src/storage/schema_bootstrap.sql",
        "schema",
    )?);
    let output_dir = tmp_output_dir_arg(
        &params.output_dir,
        "/tmp/cynic-storage-primary",
        "output_dir",
    )?;
    let plan = cynic_host::storage_primary::build_plan(&registry, &schema);
    let report = cynic_host::storage_primary::write_activation_bundle(
        &plan,
        std::path::Path::new(&output_dir),
    )
    .map_err(|error| McpError::internal_error(error, None))?;
    serde_json::to_value(report).map_err(|error| McpError::internal_error(error.to_string(), None))
}

fn storage_primary_readiness_status_params(
    params: &StoragePrimaryReadinessParams,
) -> StoragePrimaryStatusParams {
    StoragePrimaryStatusParams {
        bundle_dir: None,
        auto_bundle_dir: Some(true),
        bundle_default_dir: params.bundle_default_dir.clone(),
        bridge_status_json: None,
        auto_bridge_status: Some(true),
        fleet_report_json: None,
        mesh_status_json: None,
        auto_mesh_status: Some(true),
        resolve_mesh_dns: Some(params.resolve_mesh_dns.unwrap_or(true)),
        auto_fleet_status: Some(true),
        health_json: None,
        auto_service_health: Some(true),
        service_health_timeout_seconds: Some(params.service_health_timeout_seconds.unwrap_or(3)),
    }
}

fn storage_primary_readiness_result(mut status: serde_json::Value) -> serde_json::Value {
    let missing = status
        .get("missing")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let ready = status
        .get("ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or_else(|| {
            missing.is_empty()
                && status.get("status").and_then(|value| value.as_str()) == Some("active")
        });
    let next_actions: Vec<serde_json::Value> = if missing.is_empty() {
        status
            .get("next_actions")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default()
    } else {
        missing
            .iter()
            .filter_map(|value| value.as_str())
            .map(|item| {
                let action = match item {
                    "bridge_active" => "activate_lxc_exec_bridge_on_proxmox_host",
                    "fleet_observes_node_running" => "start_or_observe_cynic_lxc_surrealdb",
                    "mesh_reaches_storage" => {
                        "activate_synapse_dns_with_network_mesh_dns_activation_bundle"
                    }
                    "bundle_complete" => "run_storage_primary_bundle",
                    "service_health_healthy" => "start_surrealdb_and_verify_synapse_health",
                    other => other,
                };
                serde_json::json!(action)
            })
            .collect()
    };
    if let Some(object) = status.as_object_mut() {
        object.insert(
            "schema".to_string(),
            serde_json::json!("cynic.storage_primary_activation.readiness.v1"),
        );
        object.insert(
            "mcp_operation".to_string(),
            serde_json::json!("storage.primary.readiness"),
        );
        object.insert("ready".to_string(), serde_json::json!(ready));
        object.insert("next_actions".to_string(), serde_json::json!(next_actions));
    }
    status
}

fn storage_primary_status_uses_auto_mesh(params: &StoragePrimaryStatusParams) -> bool {
    params.mesh_status_json.is_none()
        && (params.auto_mesh_status.unwrap_or(false) || params.resolve_mesh_dns.is_some())
}

fn storage_primary_status_uses_auto_bundle(params: &StoragePrimaryStatusParams) -> bool {
    params.bundle_dir.is_none() && params.auto_bundle_dir.unwrap_or(true)
}

fn storage_primary_status_bundle_dir(
    params: &StoragePrimaryStatusParams,
) -> Result<Option<String>, McpError> {
    if let Some(value) = &params.bundle_dir {
        return Ok(Some(value.clone()));
    }
    if !storage_primary_status_uses_auto_bundle(params) {
        return Ok(None);
    }
    Ok(Some(tmp_output_dir_arg(
        &params.bundle_default_dir,
        "/tmp/cynic-storage-primary",
        "bundle_default_dir",
    )?))
}

fn storage_primary_status_uses_auto_bridge(params: &StoragePrimaryStatusParams) -> bool {
    params.bridge_status_json.is_none() && params.auto_bridge_status.unwrap_or(false)
}

fn storage_primary_status_uses_auto_fleet(params: &StoragePrimaryStatusParams) -> bool {
    params.fleet_report_json.is_none() && params.auto_fleet_status.unwrap_or(false)
}

fn storage_primary_status_uses_auto_health(params: &StoragePrimaryStatusParams) -> bool {
    params.health_json.is_none() && params.auto_service_health.unwrap_or(false)
}

fn run_storage_primary_status(
    params: &StoragePrimaryStatusParams,
    evidence: Option<serde_json::Value>,
) -> Result<serde_json::Value, McpError> {
    let _ = storage_primary_status_bundle_dir(params)?;
    let registry = project_root().join("infra/registry.json");
    let schema = project_root().join("crates/cynic-kernel/src/storage/schema_bootstrap.sql");
    let plan = cynic_host::storage_primary::build_plan(&registry, &schema);
    let report = cynic_host::storage_primary::evaluate_status(
        &plan,
        evidence.unwrap_or_else(|| serde_json::json!({})),
    );
    serde_json::to_value(report).map_err(|error| McpError::internal_error(error.to_string(), None))
}

#[cfg(test)]
fn storage_primary_status_args(
    params: &StoragePrimaryStatusParams,
    evidence_stdin: bool,
) -> Result<Vec<String>, McpError> {
    let mut args = vec!["storage-primary".to_string(), "status".to_string()];
    if let Some(value) = storage_primary_status_bundle_dir(params)? {
        args.extend(["--bundle-dir".to_string(), value.clone()]);
    }
    if let Some(value) = &params.bridge_status_json {
        args.extend(["--bridge-status-json".to_string(), value.clone()]);
    }
    if let Some(value) = &params.fleet_report_json {
        args.extend(["--fleet-report-json".to_string(), value.clone()]);
    }
    if let Some(value) = &params.mesh_status_json {
        args.extend(["--mesh-status-json".to_string(), value.clone()]);
    }
    if let Some(value) = &params.health_json {
        args.extend(["--health-json".to_string(), value.clone()]);
    }
    if evidence_stdin {
        args.push("--evidence-stdin".to_string());
    }
    Ok(args)
}

fn storage_fleet_report(
    registry: &serde_json::Value,
    provider_payload: &serde_json::Value,
) -> Result<serde_json::Value, McpError> {
    let node = find_registry_node(registry, "cynic-lxc-surrealdb").ok_or_else(|| {
        McpError::internal_error("storage primary node missing from registry", None)
    })?;
    let declared_status = node
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown");
    let observed = observed_resource_for_node(registry, node, provider_payload);
    let observed_status = observed
        .as_ref()
        .and_then(|value| value.get("status"))
        .and_then(|value| value.as_str());
    Ok(serde_json::json!({
        "schema": "cynic.infrastructure_mcp.storage_fleet_report.v1",
        "source": "fleet.node.status",
        "effective": [{
            "name": node_name(node),
            "declared_status": declared_status,
            "observed_status": observed_status,
            "effective_status": effective_status_from_provider(declared_status, observed_status),
            "observed": observed,
        }],
    }))
}
