fn lxc_exec_bridge_helper_source_arg(value: &Option<String>) -> String {
    value
        .clone()
        .unwrap_or_else(|| "unsupported/minimal-baseline/lxc_exec_bridge_helper".to_string())
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

fn run_organ_fleet_json(_args: &[String], operation: &str) -> Result<serde_json::Value, McpError> {
    Ok(unsupported_minimal_baseline(
        operation,
        "legacy organ-fleet Python CLI was removed from the low-layer baseline",
    ))
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
