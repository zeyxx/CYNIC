pub(super) fn render_node_control_config(
    registry: &serde_json::Value,
    target_role: &str,
) -> Result<Option<String>, String> {
    if target_role == "unassigned" {
        return Ok(None);
    }
    let nodes = registry
        .get("nodes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "runtime registry nodes are absent".to_string())?;
    let matches = nodes
        .iter()
        .filter(|node| node.get("role").and_then(serde_json::Value::as_str) == Some(target_role))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!(
            "runtime registry must select exactly one node for role {target_role}, observed {}",
            matches.len()
        ));
    }
    let node = matches[0];
    let node_id = node
        .get("id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("runtime node for role {target_role} has no id"))?;
    let observed_services = node
        .get("services")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|service| service.get("name").and_then(serde_json::Value::as_str))
        .map(ToString::to_string)
        .collect();
    let capabilities = vec!["node_control".to_string(), target_role.to_string()];
    let node_id = toml_string(node_id)?;
    let target_role = toml_string(target_role)?;
    let capabilities = toml_string_array(&capabilities)?;
    let observed_services = toml_string_array(&observed_services)?;
    Ok(Some(format!(
        "[node_control]\n\
kernel_url = \"${{CYNIC_KERNEL_URL}}\"\n\
node_id = {node_id}\n\
signing_key_path = \"/var/lib/cynic/identity/node-signing.key\"\n\
enrollment_request_path = \"/var/lib/cynic/identity/enrollment-request.json\"\n\
enrollment_grant_path = \"/var/lib/cynic/identity/enrollment-grant.json\"\n\
enrollment_authority_public_key_path = \"/etc/cynic/enrollment-authority.pub\"\n\
enrollment_token_path = \"/var/lib/cynic/identity/enrollment-token\"\n\
enrollment_seed_id = \"${{CYNIC_ENROLLMENT_SEED_ID}}\"\n\
enrollment_role = {target_role}\n\
enrollment_requested_capabilities = {capabilities}\n\
observed_capabilities = {capabilities}\n\
observed_services = {observed_services}\n"
    )))
}

fn toml_string(value: &str) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| format!("failed to render TOML string: {error}"))
}

fn toml_string_array(values: &[String]) -> Result<String, String> {
    values
        .iter()
        .map(|value| toml_string(value))
        .collect::<Result<Vec<_>, _>>()
        .map(|values| format!("[{}]", values.join(", ")))
}
