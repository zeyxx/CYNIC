fn load_infra_registry() -> Result<serde_json::Value, McpError> {
    let full_path = project_root().join("infra/registry.json");
    let content = std::fs::read_to_string(&full_path).map_err(|e| {
        McpError::internal_error(
            format!("Infrastructure registry unavailable ({full_path:?}): {e}"),
            None,
        )
    })?;
    serde_json::from_str(&content).map_err(|e| {
        McpError::internal_error(
            format!("Infrastructure registry invalid JSON ({full_path:?}): {e}"),
            None,
        )
    })
}

fn registry_selector<'a>(
    registry: &'a serde_json::Value,
    selector: Option<&str>,
) -> Option<&'a serde_json::Value> {
    let Some(selector) = selector else {
        return Some(registry);
    };
    if selector.trim().is_empty() {
        return Some(registry);
    }
    let mut current = registry;
    for part in selector.split('.').filter(|part| !part.is_empty()) {
        current = current.get(part)?;
    }
    Some(current)
}

fn registry_nodes(registry: &serde_json::Value) -> Vec<&serde_json::Value> {
    registry
        .get("nodes")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .collect()
}

fn node_name(node: &serde_json::Value) -> Option<&str> {
    node.get("name").and_then(|value| value.as_str())
}

fn node_aliases(node: &serde_json::Value) -> Vec<&str> {
    node.get("aliases")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .collect()
}

fn node_matches_directly(node: &serde_json::Value, query: &str) -> bool {
    node_name(node) == Some(query) || node_aliases(node).contains(&query)
}

fn node_matches_nested_vm(node: &serde_json::Value, query: &str) -> bool {
    node.get("vms")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .any(|vm| {
            vm.get("id").and_then(|value| value.as_str()) == Some(query)
                || vm.get("name").and_then(|value| value.as_str()) == Some(query)
                || vm
                    .get("aliases")
                    .and_then(|value| value.as_array())
                    .into_iter()
                    .flatten()
                    .any(|alias| alias.as_str() == Some(query))
        })
}

fn find_registry_node<'a>(
    registry: &'a serde_json::Value,
    query: &str,
) -> Option<&'a serde_json::Value> {
    let nodes = registry_nodes(registry);
    nodes
        .iter()
        .copied()
        .find(|node| node_matches_directly(node, query))
        .or_else(|| {
            nodes
                .iter()
                .copied()
                .find(|node| node_matches_nested_vm(node, query))
        })
}

fn find_declared_service<'a>(
    node: &'a serde_json::Value,
    service_name: &str,
) -> Option<&'a serde_json::Value> {
    node.get("services")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .find(|service| service.get("name").and_then(|value| value.as_str()) == Some(service_name))
}

fn service_endpoint_from_registry(
    node: &serde_json::Value,
    service: &serde_json::Value,
) -> Option<String> {
    let port = service.get("port").and_then(|value| value.as_u64())?;
    let host = node_aliases(node)
        .into_iter()
        .find(|alias| alias.starts_with("organ-"))
        .or_else(|| node_name(node))?;
    Some(format!("http://{host}.synapse.cynic:{port}/health"))
}
