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
        .collect::<Vec<String>>();
    let capabilities = cynic_protocol::required_node_enrollment_capabilities(target_role);
    let build_scheduler_enabled = target_role == "build_runner";
    let node_id = toml_string(node_id)?;
    let target_role = toml_string(target_role)?;
    let capabilities = toml_string_array(&capabilities)?;
    let observed_services = toml_string_array(&observed_services)?;
    let build_scheduler = if build_scheduler_enabled {
        let release_publication = render_release_publication_config(registry)?;
        format!(
            "\n[node_control.build_scheduler]\nregistry_path = \"/etc/cynic/registry.json\"\nrepository_id = \"cynic\"\n{release_publication}"
        )
    } else {
        String::new()
    };
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
agent_workload_state_root = \"${{CYNIC_STATE_ROOT}}\"\n\
runtime_artifact_inbox_root = \"${{CYNIC_STATE_ROOT}}/runtime-artifacts/inbox\"\n\
runtime_artifact_max_bytes = 4294967296\n\
runtime_artifact_timeout_secs = 900\n\
enrollment_requested_capabilities = {capabilities}\n\
observed_capabilities = {capabilities}\n\
observed_services = {observed_services}\n\
{build_scheduler}"
    )))
}

fn render_release_publication_config(registry: &serde_json::Value) -> Result<String, String> {
    let desired = registry
        .get("release_publication")
        .cloned()
        .ok_or_else(|| "release publication desired state is absent".to_string())?;
    let desired = serde_json::from_value::<cynic_build::ReleasePublicationDesiredState>(desired)
        .map_err(|_| "release publication desired state is invalid".to_string())?;
    let blockers = cynic_build::validate_release_publication_desired_state(&desired);
    if !blockers.is_empty() {
        return Err(blockers.join(","));
    }
    let nodes = registry
        .get("nodes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "runtime registry nodes are absent".to_string())?;
    let storage_nodes = nodes
        .iter()
        .filter(|node| {
            node.get("role").and_then(serde_json::Value::as_str)
                == Some(desired.storage_role.as_str())
        })
        .collect::<Vec<_>>();
    let [storage_node] = storage_nodes.as_slice() else {
        return Err(format!(
            "release publication must resolve exactly one storage node for role {}",
            desired.storage_role
        ));
    };
    let proxmox_id = storage_node
        .get("proxmox_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if !proxmox_id.starts_with("lxc/") && !proxmox_id.starts_with("vm/") {
        return Err("release publication storage placement is not Proxmox-backed".to_string());
    }
    let services = storage_node
        .get("services")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "release publication storage services are absent".to_string())?;
    let services = services
        .iter()
        .filter(|service| {
            service.get("name").and_then(serde_json::Value::as_str)
                == Some(desired.service_name.as_str())
        })
        .collect::<Vec<_>>();
    let [service] = services.as_slice() else {
        return Err(format!(
            "release publication must resolve exactly one service named {}",
            desired.service_name
        ));
    };
    if service.get("scheme").and_then(serde_json::Value::as_str) != Some("https")
        || service.get("protocol").and_then(serde_json::Value::as_str)
            != Some(desired.protocol.as_str())
        || service
            .get("authentication_kind")
            .and_then(serde_json::Value::as_str)
            != Some(desired.authentication_kind.as_str())
    {
        return Err("release publication storage service contract mismatch".to_string());
    }
    let host = service
        .get("endpoint_host")
        .and_then(serde_json::Value::as_str)
        .filter(|host| safe_host(host))
        .ok_or_else(|| "release publication endpoint host is invalid".to_string())?;
    let port = service
        .get("port")
        .and_then(serde_json::Value::as_u64)
        .filter(|port| (1..=u64::from(u16::MAX)).contains(port))
        .ok_or_else(|| "release publication endpoint port is invalid".to_string())?;
    let base_path = service
        .get("base_path")
        .and_then(serde_json::Value::as_str)
        .filter(|path| safe_base_path(path))
        .ok_or_else(|| "release publication base path is invalid".to_string())?;
    let artifact_base_url = format!("https://{host}:{port}{base_path}");
    Ok(format!(
        "\n[node_control.build_scheduler.release_publication]\nartifact_base_url = {}\nauthentication_kind = {}\nwork_root = \"${{CYNIC_STATE_ROOT}}/release-publication\"\nsigning_key_path = {}\ncredential_path = {}\nsigner_identity = {}\nsignature_namespace = {}\nmax_bytes = 4294967296\ntimeout_secs = 900\n",
        toml_string(&artifact_base_url)?,
        toml_string(&desired.authentication_kind)?,
        toml_string(&desired.signing_key_path)?,
        toml_string(&desired.credential_path)?,
        toml_string(&desired.signer_identity)?,
        toml_string(&desired.signature_namespace)?,
    ))
}

fn safe_host(host: &str) -> bool {
    !host.is_empty()
        && !host.starts_with('-')
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'))
}

fn safe_base_path(path: &str) -> bool {
    path.starts_with('/')
        && path.len() > 1
        && !path.contains("..")
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.'))
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
