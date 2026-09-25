fn proxmox_resource_id_for_node(
    registry: &serde_json::Value,
    node: &serde_json::Value,
) -> Option<String> {
    let name = node_name(node)?;
    for host in registry_nodes(registry) {
        for vm in host
            .get("vms")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
        {
            let vm_name = vm.get("name").and_then(|value| value.as_str());
            let alias_match = vm
                .get("aliases")
                .and_then(|value| value.as_array())
                .into_iter()
                .flatten()
                .any(|alias| alias.as_str() == Some(name));
            if vm_name == Some(name) || alias_match {
                return vm
                    .get("id")
                    .and_then(|value| value.as_str())
                    .map(String::from);
            }
        }
    }
    None
}

fn proxmox_resource_kind_id(proxmox_id: &str) -> Option<(&str, &str)> {
    let (kind, id) = proxmox_id.split_once('/')?;
    if matches!(kind, "qemu" | "lxc") && !id.is_empty() {
        Some((kind, id))
    } else {
        None
    }
}

fn node_requires_host_exec_transport(node: &serde_json::Value) -> bool {
    matches!(
        node.get("role").and_then(|value| value.as_str()),
        Some("cortex_host") | Some("inference_node") | Some("hypervisor")
    )
}

fn host_exec_transport_for_node(node: &serde_json::Value) -> Option<&serde_json::Value> {
    node.get("host_exec_transport").filter(|value| value.is_object())
}

fn host_exec_transport_kind(node: &serde_json::Value) -> Option<&str> {
    host_exec_transport_for_node(node)?
        .get("kind")
        .and_then(|value| value.as_str())
}

fn host_exec_transport_command_argv(
    node: &serde_json::Value,
) -> Result<Option<Vec<String>>, String> {
    let Some(transport) = host_exec_transport_for_node(node) else {
        return Ok(None);
    };
    let env_name = transport
        .get("transport_command_env")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let default_command = transport
        .get("transport_command_default")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let configured = if !env_name.is_empty() {
        std::env::var(env_name).unwrap_or_else(|_| default_command.to_string())
    } else {
        default_command.to_string()
    };
    if configured.trim().is_empty() {
        return Ok(None);
    }
    let argv: Vec<String> = configured
        .split_whitespace()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(String::from)
        .collect();
    if argv.is_empty() {
        return Err("host_exec_transport command resolved to an empty argv".to_string());
    }
    Ok(Some(argv))
}

fn observed_resource_for_node(
    registry: &serde_json::Value,
    node: &serde_json::Value,
    provider_payload: &serde_json::Value,
) -> Option<serde_json::Value> {
    let proxmox_id = proxmox_resource_id_for_node(registry, node);
    provider_payload
        .get("resources")
        .and_then(|value| value.as_array())
        .and_then(|resources| {
            resources.iter().find(|resource| {
                let resource_id = resource.get("id").and_then(|value| value.as_str());
                let resource_name = resource.get("name").and_then(|value| value.as_str());
                resource_id == proxmox_id.as_deref() || resource_name == node_name(node)
            })
        })
        .cloned()
}

fn effective_status_from_provider(declared_status: &str, observed_status: Option<&str>) -> String {
    if matches!(
        declared_status,
        "planned" | "planned_obsolescence" | "archived"
    ) {
        return declared_status.to_string();
    }
    match observed_status {
        Some("running") => "active".to_string(),
        Some("stopped") => "stopped".to_string(),
        Some(status) => status.to_string(),
        None if declared_status == "active" => "stale".to_string(),
        None => declared_status.to_string(),
    }
}
