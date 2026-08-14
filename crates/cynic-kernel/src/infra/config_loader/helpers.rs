use cynic_host::runtime_registry::resolve_runtime_registry_path;

pub(crate) fn load_file(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

pub(crate) fn resolve_registry_placeholders(mut content: String) -> String {
    let Ok(registry_path) = resolve_runtime_registry_path(None) else {
        return content;
    };

    let Ok(registry_content) = std::fs::read_to_string(&registry_path) else {
        return content;
    };

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&registry_content) else {
        return content;
    };

    let Some(nodes) = json.get("nodes").and_then(|n| n.as_array()) else {
        return content;
    };

    for node in nodes {
        let Some(name) = node.get("name").and_then(|n| n.as_str()) else {
            continue;
        };
        let name_upper = if let Some(stripped) = name.strip_prefix("cynic-") {
            stripped.to_uppercase().replace('-', "_")
        } else {
            name.to_uppercase().replace('-', "_")
        };

        if let Some(ts) = node
            .get("network")
            .and_then(|nw| nw.get("tailscale"))
            .and_then(|t| t.as_str())
        {
            let key = format!("<TAILSCALE_{name_upper}>");
            let resolved = if ts.starts_with('<') && ts.ends_with('>') {
                let env_var = &ts[1..ts.len() - 1];
                std::env::var(env_var).unwrap_or_else(|_| ts.to_string())
            } else {
                ts.to_string()
            };
            content = content.replace(&key, &resolved);
        }

        if let Some(local) = node
            .get("network")
            .and_then(|nw| nw.get("local"))
            .and_then(|t| t.as_str())
        {
            let key = format!("<LOCAL_{name_upper}>");
            content = content.replace(&key, local);
        }
    }

    content
}
