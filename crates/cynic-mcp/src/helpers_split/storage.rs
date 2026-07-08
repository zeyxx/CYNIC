fn organ_keep_cli_path() -> PathBuf {
    project_root().join(
        std::env::var("CYNIC_ORGAN_KEEP_CLI_PATH")
            .unwrap_or_else(|_| "organs/keep/src/entrypoints/cli.py".to_string()),
    )
}

fn organ_keep_cli_command() -> String {
    std::env::var("CYNIC_ORGAN_KEEP_CLI_CMD").unwrap_or_else(|_| "python3".to_string())
}

fn storage_primary_plan_args(params: &StoragePrimaryPlanParams) -> Result<Vec<String>, McpError> {
    Ok(vec![
        "storage-primary".to_string(),
        "plan".to_string(),
        "--registry".to_string(),
        repo_relative_arg(&params.registry, "infra/registry.json", "registry")?,
    ])
}

fn storage_primary_bundle_args(
    params: &StoragePrimaryBundleParams,
) -> Result<Vec<String>, McpError> {
    Ok(vec![
        "storage-primary".to_string(),
        "bundle".to_string(),
        "--registry".to_string(),
        repo_relative_arg(&params.registry, "infra/registry.json", "registry")?,
        "--schema".to_string(),
        repo_relative_arg(&params.schema, "infra/surrealdb/schema.surql", "schema")?,
        "--output-dir".to_string(),
        tmp_output_dir_arg(
            &params.output_dir,
            "/tmp/cynic-storage-primary",
            "output_dir",
        )?,
    ])
}

fn run_organ_keep_json(args: &[String], operation: &str) -> Result<serde_json::Value, McpError> {
    let cli_path = organ_keep_cli_path();
    let cli_command = organ_keep_cli_command();
    let output = std::process::Command::new(&cli_command)
        .arg(&cli_path)
        .args(args)
        .output()
        .map_err(|e| {
            McpError::internal_error(
                format!(
                    "Failed to run organ-keep CLI command ({}) {}: {e}",
                    cli_command,
                    cli_path.display()
                ),
                None,
            )
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut payload: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
        McpError::internal_error(
            format!(
                "organ-keep {operation} did not emit valid JSON: {e}; stderr={}",
                stderr.trim()
            ),
            None,
        )
    })?;
    if let Some(object) = payload.as_object_mut() {
        object.insert("mcp_operation".to_string(), serde_json::json!(operation));
        object.insert(
            "cli_exit_code".to_string(),
            serde_json::json!(output.status.code()),
        );
    }
    Ok(payload)
}

fn run_storage_primary_plan(
    params: &StoragePrimaryPlanParams,
) -> Result<serde_json::Value, McpError> {
    let args = storage_primary_plan_args(params)?;
    run_organ_keep_json(&args, "storage.primary.plan")
}

fn run_storage_primary_bundle(
    params: &StoragePrimaryBundleParams,
) -> Result<serde_json::Value, McpError> {
    let args = storage_primary_bundle_args(params)?;
    run_organ_keep_json(&args, "storage.primary.bundle")
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
    let ready = missing.is_empty()
        && status.get("status").and_then(|value| value.as_str()) == Some("active");
    let next_actions: Vec<serde_json::Value> = missing
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
        .collect();
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

fn run_storage_primary_status(
    params: &StoragePrimaryStatusParams,
    evidence: Option<serde_json::Value>,
) -> Result<serde_json::Value, McpError> {
    let cli_path = organ_keep_cli_path();
    let cli_command = organ_keep_cli_command();
    let use_evidence_stdin = evidence.is_some();
    let output = if let Some(evidence) = evidence {
        let mut child = std::process::Command::new(&cli_command)
            .arg(&cli_path)
            .args(storage_primary_status_args(params, true)?)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                McpError::internal_error(
                    format!(
                        "Failed to spawn organ-keep CLI command ({}) {}: {e}",
                        cli_command,
                        cli_path.display()
                    ),
                    None,
                )
            })?;
        let stdin = child.stdin.as_mut().ok_or_else(|| {
            McpError::internal_error("Failed to open stdin for organ-keep CLI".to_string(), None)
        })?;
        let evidence_json = serde_json::to_string(&evidence).map_err(|e| {
            McpError::internal_error(
                format!("Failed to serialize storage status evidence for organ-keep CLI: {e}"),
                None,
            )
        })?;
        stdin.write_all(evidence_json.as_bytes()).map_err(|e| {
            McpError::internal_error(
                format!("Failed to send storage status evidence to organ-keep CLI: {e}"),
                None,
            )
        })?;
        child.wait_with_output().map_err(|e| {
            McpError::internal_error(
                format!(
                    "Failed to wait for organ-keep CLI command ({}) {}: {e}",
                    cli_command,
                    cli_path.display()
                ),
                None,
            )
        })?
    } else {
        std::process::Command::new(&cli_command)
            .arg(&cli_path)
            .args(storage_primary_status_args(params, false)?)
            .output()
            .map_err(|e| {
                McpError::internal_error(
                    format!(
                        "Failed to run organ-keep CLI command ({}) {}: {e}",
                        cli_command,
                        cli_path.display()
                    ),
                    None,
                )
            })?
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut payload: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
        McpError::internal_error(
            format!(
                "organ-keep storage-primary status did not emit valid JSON: {e}; stderr={}",
                stderr.trim()
            ),
            None,
        )
    })?;
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "mcp_operation".to_string(),
            serde_json::json!("storage.primary.status"),
        );
        object.insert(
            "cli_exit_code".to_string(),
            serde_json::json!(output.status.code()),
        );
        object.insert(
            "bundle_evidence_source".to_string(),
            serde_json::json!(if params.bundle_dir.is_some() {
                "explicit_bundle_dir"
            } else if storage_primary_status_uses_auto_bundle(params) {
                "default_bundle_dir"
            } else {
                "none"
            }),
        );
        object.insert(
            "mesh_evidence_source".to_string(),
            serde_json::json!(if storage_primary_status_uses_auto_mesh(params) {
                "mcp_auto_evidence_stdin"
            } else if params.mesh_status_json.is_some() {
                "json_file"
            } else {
                "none"
            }),
        );
        object.insert(
            "bridge_evidence_source".to_string(),
            serde_json::json!(if storage_primary_status_uses_auto_bridge(params) {
                "mcp_auto_evidence_stdin"
            } else if params.bridge_status_json.is_some() {
                "json_file"
            } else {
                "none"
            }),
        );
        object.insert(
            "fleet_evidence_source".to_string(),
            serde_json::json!(if storage_primary_status_uses_auto_fleet(params) {
                "mcp_auto_evidence_stdin"
            } else if params.fleet_report_json.is_some() {
                "json_file"
            } else {
                "none"
            }),
        );
        object.insert(
            "health_evidence_source".to_string(),
            serde_json::json!(if storage_primary_status_uses_auto_health(params) {
                "mcp_auto_evidence_stdin"
            } else if params.health_json.is_some() {
                "json_file"
            } else {
                "none"
            }),
        );
        object.insert(
            "evidence_stdin".to_string(),
            serde_json::json!(use_evidence_stdin),
        );
    }
    Ok(payload)
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
