pub fn validate_agent_id(agent_id: &Option<String>) -> Result<(), rmcp::ErrorData> {
    if let Some(id) = agent_id {
        if id.is_empty() || id.chars().count() > 64 {
            return Err(rmcp::ErrorData::invalid_params(
                "agent_id must be 1-64 characters",
                None,
            ));
        }
        if id.chars().any(|c| c.is_control()) {
            return Err(rmcp::ErrorData::invalid_params(
                "agent_id contains invalid characters",
                None,
            ));
        }
    }
    Ok(())
}

pub fn validate_agent_task_content(
    kind: &str,
    domain: &str,
    content: &str,
) -> Result<(), rmcp::ErrorData> {
    cynic_host::lifecycle::forge_task_contract::validate_repo_affecting_hermes_task(
        kind,
        domain,
        content,
        load_registry().as_ref(),
    )
    .map_err(|message| rmcp::ErrorData::invalid_params(message, None))?;
    Ok(())
}

fn load_registry() -> Option<serde_json::Value> {
    cynic_host::runtime_registry::load_runtime_registry(None)
}
