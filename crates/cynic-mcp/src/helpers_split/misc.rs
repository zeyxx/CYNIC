fn network_surface_cli_path() -> PathBuf {
    project_root().join(
        std::env::var("CYNIC_NETWORK_SURFACE_CLI_PATH")
            .or_else(|_| std::env::var("CYNIC_ORGAN_WIREGUARD_CLI_PATH"))
            .unwrap_or_else(|_| "organs/fleet/src/entrypoints/cli.py".to_string()),
    )
}

fn network_surface_cli_command() -> String {
    std::env::var("CYNIC_NETWORK_SURFACE_CLI_CMD")
        .or_else(|_| std::env::var("CYNIC_ORGAN_WIREGUARD_CLI_CMD"))
        .unwrap_or_else(|_| "python3".to_string())
}

fn repo_relative_arg(
    value: &Option<String>,
    default: &str,
    field: &str,
) -> Result<String, McpError> {
    let resolved = value.clone().unwrap_or_else(|| default.to_string());
    let path = PathBuf::from(&resolved);
    if path.is_absolute() || resolved.contains("..") {
        return Err(McpError::invalid_params(
            format!("{field} must be a project-relative path without '..'"),
            None,
        ));
    }
    Ok(resolved)
}

fn tmp_output_dir_arg(
    value: &Option<String>,
    default: &str,
    field: &str,
) -> Result<String, McpError> {
    let output_dir = value.clone().unwrap_or_else(|| default.to_string());
    let path = PathBuf::from(&output_dir);
    if !path.is_absolute() || !path.starts_with("/tmp") {
        return Err(McpError::invalid_params(
            format!("{field} must be an absolute path under /tmp"),
            None,
        ));
    }
    Ok(output_dir)
}

#[cfg_attr(not(feature = "legacy_infra_tools"), allow(dead_code))]
fn require_legacy_infra_tool(tool_name: &str) -> Result<(), McpError> {
    const LEGACY_INFRA_TOOL_ENV: &str = "CYNIC_ALLOW_LEGACY_INFRA_TOOLS";
    if std::env::var(LEGACY_INFRA_TOOL_ENV).unwrap_or_default() == "1" {
        return Ok(());
    }

    Err(McpError::invalid_request(
        format!(
            "legacy provider tool '{tool_name}' is disabled by policy. Set {LEGACY_INFRA_TOOL_ENV}=1 for breakglass access only, or use fleet_node_exec/fleet_node_status for canonical operations.",
        ),
        None,
    ))
}

fn registry_ref_arg(value: &Option<String>) -> Result<String, McpError> {
    let registry_ref = value
        .clone()
        .unwrap_or_else(|| "infra/registry.json".to_string());
    if registry_ref != "infra/registry.json" {
        return Err(McpError::invalid_params(
            "v1 only supports registry_ref=infra/registry.json",
            None,
        ));
    }
    Ok(registry_ref)
}

fn run_network_surface_json(
    args: &[String],
    operation: &str,
) -> Result<serde_json::Value, McpError> {
    let cli_path = network_surface_cli_path();
    let cli_command = network_surface_cli_command();
    let output = std::process::Command::new(&cli_command)
        .arg(&cli_path)
        .args(args)
        .output()
        .map_err(|e| {
            McpError::internal_error(
                format!(
                    "Failed to run network surface CLI command ({}) {}: {e}",
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
                "network surface {operation} did not emit valid JSON: {e}; stderr={}",
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

const COORDINATION_DOMAIN: &str = "coordination";
const COORDINATION_TOOL: &str = "coordination";
const COORDINATION_SCHEMA: &str = "coordination-event-v1";
const DEFAULT_COORDINATION_PROTOCOL_PATH: &str = "axioms/coordination-protocol-v1.json";
const DEFAULT_COORDINATION_CLIENT_PROFILE_DIR: &str = "axioms/consumer-profiles";
const COORDINATION_OPERATION_STATUS_PUBLISH: &str = "coordination.status.publish";
const COORDINATION_OPERATION_STATUS_READ: &str = "coordination.status.read";
const COORDINATION_OPERATION_THREAD_REPLY: &str = "coordination.thread.reply";
const COORDINATION_OPERATION_ARTIFACT_SHARE: &str = "coordination.artifact.share";
const COORDINATION_OPERATION_DECISION_RECORD: &str = "coordination.decision.record";

fn project_root() -> PathBuf {
    std::env::var("CYNIC_PROJECT_ROOT")
        .map(PathBuf::from)
        .ok()
        .or_else(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .output()
                .ok()
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|path| PathBuf::from(path.trim()))
        })
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn load_consumer_profile(project: &str) -> Result<Option<serde_json::Value>, McpError> {
    let filename = format!("{project}-v1.json");
    let path = project_root()
        .join(coordination_client_profile_dir())
        .join(filename);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path).map_err(|e| {
        McpError::internal_error(
            format!("Consumer coordination profile file unavailable ({path:?}): {e}"),
            None,
        )
    })?;
    serde_json::from_str(&content).map(Some).map_err(|e| {
        McpError::internal_error(
            format!("Consumer coordination profile file invalid JSON ({path:?}): {e}"),
            None,
        )
    })
}

async fn service_health_report(
    node: &serde_json::Value,
    service_name: &str,
    service: &serde_json::Value,
    endpoint: Option<String>,
    timeout_seconds: Option<u64>,
) -> Result<serde_json::Value, McpError> {
    let endpoint = endpoint.or_else(|| service_endpoint_from_registry(node, service));
    let Some(endpoint) = endpoint else {
        return Ok(serde_json::json!({
            "schema": "cynic.infrastructure_mcp.service_health.v1",
            "node": node_name(node),
            "service": service_name,
            "healthy": false,
            "endpoint": serde_json::Value::Null,
            "failure_reason": "service_has_no_declared_port_or_endpoint",
        }));
    };
    if !endpoint.contains(".synapse.cynic") {
        return Err(McpError::invalid_params(
            "service_health endpoint must use the Synapse service contract in v1",
            None,
        ));
    }
    let timeout = std::time::Duration::from_secs(timeout_seconds.unwrap_or(5).clamp(1, 30));
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| McpError::internal_error(format!("Health client error: {e}"), None))?;
    let start = std::time::Instant::now();
    let response = client.get(&endpoint).send().await;
    let latency_ms = start.elapsed().as_millis();
    Ok(match response {
        Ok(resp) => {
            let status = resp.status().as_u16();
            serde_json::json!({
                "schema": "cynic.infrastructure_mcp.service_health.v1",
                "node": node_name(node),
                "service": service_name,
                "healthy": status < 400,
                "endpoint": endpoint,
                "http_status": status,
                "latency_ms": latency_ms,
                "failure_reason": if status < 400 { serde_json::Value::Null } else { serde_json::json!(format!("http_status_{status}")) },
            })
        }
        Err(e) => serde_json::json!({
            "schema": "cynic.infrastructure_mcp.service_health.v1",
            "node": node_name(node),
            "service": service_name,
            "healthy": false,
            "endpoint": endpoint,
            "latency_ms": latency_ms,
            "failure_reason": format!("request_failed: {e}"),
        }),
    })
}

fn find_reservation<'a>(
    registry: &'a serde_json::Value,
    node_name: &str,
) -> Option<&'a serde_json::Value> {
    network_reservations(registry)
        .into_iter()
        .find(|reservation| {
            reservation.get("name").and_then(|value| value.as_str()) == Some(node_name)
        })
}

fn query_url(base_url: &str, path: &str, params: &[(&str, String)]) -> Result<String, McpError> {
    let mut url = reqwest::Url::parse(&format!("{base_url}{path}")).map_err(|e| {
        McpError::internal_error(format!("REST kernel base URL is invalid: {e}"), None)
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in params {
            pairs.append_pair(key, value);
        }
    }
    Ok(url.to_string())
}
