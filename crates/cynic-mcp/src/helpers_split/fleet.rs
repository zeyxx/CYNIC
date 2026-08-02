fn lxc_exec_bridge_helper_source_arg(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| {
        std::env::current_exe()
            .ok()
            .map(|path| path.with_file_name("cynic-lxc-exec-bridge"))
            .unwrap_or_else(|| PathBuf::from("cynic-lxc-exec-bridge"))
            .display()
            .to_string()
    })
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

fn run_lxc_exec_bridge_plan(
    params: &LxcExecBridgePlanParams,
) -> Result<serde_json::Value, McpError> {
    serde_json::to_value(cynic_host::lxc_exec_bridge::install_plan(
        lxc_exec_bridge_helper_source_arg(&params.helper_source),
    ))
    .map_err(|error| McpError::internal_error(error.to_string(), None))
}

fn run_lxc_exec_bridge_bundle(
    params: &LxcExecBridgeBundleParams,
) -> Result<serde_json::Value, McpError> {
    let output_dir = lxc_exec_bridge_bundle_output_dir(params)?;
    serde_json::to_value(cynic_host::lxc_exec_bridge::bundle(
        output_dir,
        lxc_exec_bridge_helper_source_arg(&params.helper_source),
    ))
    .map_err(|error| McpError::internal_error(error.to_string(), None))
}

fn run_lxc_exec_bridge_status(
    params: &LxcExecBridgeStatusParams,
) -> Result<serde_json::Value, McpError> {
    let mut payload = serde_json::to_value(cynic_host::lxc_exec_bridge::status(
        params.helper_path.as_deref(),
        params
            .sudoers_path
            .as_deref()
            .or(params.installed_policy_path.as_deref()),
    ))
    .map_err(|error| McpError::internal_error(error.to_string(), None))?;
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

fn validate_lxc_argv(argv: &[String]) -> Result<Vec<String>, String> {
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
    Ok(argv.to_vec())
}
