fn organ_fleet_cli_path() -> PathBuf {
    project_root().join(
        std::env::var("CYNIC_ORGAN_FLEET_CLI_PATH")
            .unwrap_or_else(|_| "organs/fleet/src/entrypoints/cli.py".to_string()),
    )
}

fn organ_fleet_cli_command() -> String {
    std::env::var("CYNIC_ORGAN_FLEET_CLI_CMD").unwrap_or_else(|_| "python3".to_string())
}

fn lxc_exec_bridge_helper_source_arg(value: &Option<String>) -> String {
    value
        .clone()
        .unwrap_or_else(|| "organs/fleet/src/entrypoints/lxc_exec_bridge_helper.py".to_string())
}

fn lxc_exec_bridge_plan_args(params: &LxcExecBridgePlanParams) -> Vec<String> {
    vec![
        "lxc-exec-bridge".to_string(),
        "install-plan".to_string(),
        "--helper-source".to_string(),
        lxc_exec_bridge_helper_source_arg(&params.helper_source),
    ]
}

fn lxc_exec_bridge_bundle_output_dir(
    params: &LxcExecBridgeBundleParams,
) -> Result<String, McpError> {
    let output_dir = params
        .output_dir
        .clone()
        .unwrap_or_else(|| "/tmp/cynic-lxc-bridge".to_string());
    let path = PathBuf::from(&output_dir);
    if !path.is_absolute() || !path.starts_with("/tmp") {
        return Err(McpError::invalid_params(
            "lxc_exec_bridge_bundle output_dir must be an absolute path under /tmp",
            None,
        ));
    }
    Ok(output_dir)
}

fn lxc_exec_bridge_bundle_args(
    params: &LxcExecBridgeBundleParams,
) -> Result<Vec<String>, McpError> {
    Ok(vec![
        "lxc-exec-bridge".to_string(),
        "bundle".to_string(),
        "--helper-source".to_string(),
        lxc_exec_bridge_helper_source_arg(&params.helper_source),
        "--output-dir".to_string(),
        lxc_exec_bridge_bundle_output_dir(params)?,
    ])
}

fn run_organ_fleet_json(args: &[String], operation: &str) -> Result<serde_json::Value, McpError> {
    let cli_path = organ_fleet_cli_path();
    let cli_command = organ_fleet_cli_command();
    let output = std::process::Command::new(&cli_command)
        .arg(&cli_path)
        .args(args)
        .output()
        .map_err(|e| {
            McpError::internal_error(
                format!(
                    "Failed to run organ-fleet CLI command ({}) {}: {e}",
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
                "organ-fleet {operation} did not emit valid JSON: {e}; stderr={}",
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

fn run_lxc_exec_bridge_plan(
    params: &LxcExecBridgePlanParams,
) -> Result<serde_json::Value, McpError> {
    run_organ_fleet_json(&lxc_exec_bridge_plan_args(params), "lxc_exec_bridge.plan")
}

fn run_lxc_exec_bridge_bundle(
    params: &LxcExecBridgeBundleParams,
) -> Result<serde_json::Value, McpError> {
    let args = lxc_exec_bridge_bundle_args(params)?;
    run_organ_fleet_json(&args, "lxc_exec_bridge.bundle")
}

fn lxc_exec_bridge_status_args(params: &LxcExecBridgeStatusParams) -> Vec<String> {
    let mut args = vec![
        "lxc-exec-bridge".to_string(),
        "activation-status".to_string(),
    ];
    if let Some(value) = &params.helper_path {
        args.extend(["--helper-path".to_string(), value.clone()]);
    }
    if let Some(value) = &params.installed_policy_path {
        args.extend(["--installed-policy-path".to_string(), value.clone()]);
    }
    if let Some(value) = &params.sudoers_path {
        args.extend(["--sudoers-path".to_string(), value.clone()]);
    }
    args
}

fn run_lxc_exec_bridge_status(
    params: &LxcExecBridgeStatusParams,
) -> Result<serde_json::Value, McpError> {
    let mut payload = run_organ_fleet_json(
        &lxc_exec_bridge_status_args(params),
        "lxc_exec_bridge.status",
    )?;
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "transport_status".to_string(),
            serde_json::json!(if CynicMcpProxy::lxc_exec_bridge_config().is_some() {
                "configured"
            } else {
                "not_configured"
            }),
        );
    }
    Ok(payload)
}

fn lxc_command_argv(command: &str) -> Result<Vec<String>, String> {
    let argv: Vec<String> = command
        .split_whitespace()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(String::from)
        .collect();
    if argv.is_empty() {
        return Err("command must not be empty".to_string());
    }
    let forbidden_tokens = ["sh", "bash", "-c"];
    if argv
        .iter()
        .any(|token| forbidden_tokens.iter().any(|forbidden| token == forbidden))
    {
        return Err("shell tokens are not allowed for LXC bridge commands".to_string());
    }
    let forbidden = ["|", "&&", ";", ">", ">>", "$(", "`"];
    if argv
        .iter()
        .any(|token| forbidden.iter().any(|pattern| token.contains(pattern)))
    {
        return Err("shell syntax is not allowed for LXC bridge commands".to_string());
    }
    Ok(argv)
}
