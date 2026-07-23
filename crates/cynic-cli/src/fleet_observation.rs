use std::io::Read;
use std::path::Path;

use cynic_node::domain::fleet_truth::ObservedResource;

pub(crate) struct ObservationInputs<'a> {
    pub(crate) proxmox_json: Option<&'a str>,
    pub(crate) proxmox_list_text: Option<&'a str>,
    pub(crate) watchdog_json: Option<&'a str>,
}

pub(crate) fn load_observed_resources(
    inputs: &ObservationInputs<'_>,
) -> Result<Vec<ObservedResource>, String> {
    let mut resources = Vec::new();

    if let Some(path) = inputs.proxmox_json {
        resources.append(&mut parse_proxmox_json(Path::new(path))?);
    }
    if let Some(path) = inputs.proxmox_list_text {
        resources.append(&mut parse_proxmox_text(path)?);
    }
    if let Some(path) = inputs.watchdog_json {
        resources.append(&mut parse_watchdog_json(Path::new(path))?);
    }

    Ok(resources)
}

fn parse_proxmox_json(path: &Path) -> Result<Vec<ObservedResource>, String> {
    let payload: serde_json::Value = serde_json::from_slice(
        &std::fs::read(path)
            .map_err(|error| format!("failed to read proxmox json {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("failed to parse proxmox json {}: {error}", path.display()))?;
    let mut resources = Vec::new();
    for item in payload
        .get("resources")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
    {
        let Some(name) = item
            .get("name")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        resources.push(ObservedResource {
            provider: "proxmox".to_string(),
            name: name.to_string(),
            status: item
                .get("status")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown")
                .to_string(),
            resource_id: item
                .get("id")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            kind: item
                .get("kind")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            constellation: None,
        });
    }
    Ok(resources)
}

fn parse_proxmox_text(path: &str) -> Result<Vec<ObservedResource>, String> {
    let text = if path == "-" {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| format!("failed to read proxmox list text from stdin: {error}"))?;
        text
    } else {
        std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read proxmox list text {path}: {error}"))?
    };
    let mut resources = Vec::new();
    let mut current_kind: Option<&str> = None;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if let Some(name) = line
            .strip_prefix("=== Node: ")
            .and_then(|value| value.strip_suffix(" ==="))
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            resources.push(ObservedResource {
                provider: "proxmox".to_string(),
                name: name.to_string(),
                status: "running".to_string(),
                resource_id: None,
                kind: Some("hypervisor".to_string()),
                constellation: None,
            });
            continue;
        }
        if line == "VMs:" {
            current_kind = Some("vm");
            continue;
        }
        if line == "Containers (LXC):" {
            current_kind = Some("lxc");
            continue;
        }
        if !line.starts_with("- ID: ") || current_kind.is_none() {
            continue;
        }
        let Some((id_part, rest)) = line.trim_start_matches("- ID: ").split_once(", Name: ") else {
            continue;
        };
        let Some((name_part, status_part)) = rest.split_once(", Status: ") else {
            continue;
        };
        let status = status_part.split(',').next().unwrap_or("unknown").trim();
        let kind = current_kind.unwrap_or("vm");
        let resource_id = format!(
            "{}/{}",
            if kind == "vm" { "qemu" } else { "lxc" },
            id_part.trim()
        );
        resources.push(ObservedResource {
            provider: "proxmox".to_string(),
            name: name_part.trim().to_string(),
            status: status.to_string(),
            resource_id: Some(resource_id),
            kind: Some(kind.to_string()),
            constellation: None,
        });
    }
    Ok(resources)
}

fn parse_watchdog_json(path: &Path) -> Result<Vec<ObservedResource>, String> {
    let payload: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).map_err(|error| {
            format!("failed to read watchdog json {}: {error}", path.display())
        })?)
        .map_err(|error| format!("failed to parse watchdog json {}: {error}", path.display()))?;
    let items = payload.as_array().cloned().unwrap_or_else(|| vec![payload]);
    let mut resources = Vec::new();
    for item in items {
        let value =
            if item.get("tool").and_then(|value| value.as_str()) == Some("cynic-node-watchdog") {
                item.get("value")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)
            } else {
                item
            };
        let Some(object) = value.as_object() else {
            continue;
        };
        if object.get("schema").and_then(|value| value.as_str())
            != Some("cynic.node_watchdog_observation.v1")
        {
            continue;
        }
        let Some(node) = object.get("node").and_then(|value| value.as_object()) else {
            continue;
        };
        let Some(name) = node
            .get("name")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let status = if object.get("status").and_then(|value| value.as_str()) == Some("ok") {
            "running"
        } else {
            "degraded"
        };
        resources.push(ObservedResource {
            provider: "cynic-node-watchdog".to_string(),
            name: name.to_string(),
            status: status.to_string(),
            resource_id: None,
            kind: Some("node_watchdog".to_string()),
            constellation: node
                .get("constellation")
                .and_then(|value| value.as_str())
                .map(str::to_string),
        });
    }
    Ok(resources)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{ObservationInputs, load_observed_resources};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_proxmox_json_resources() {
        let path = temp_file_path("cynic-cli-proxmox-json", ".json");
        fs::write(
            &path,
            r#"{"resources":[{"name":"cynic-vm-v2","status":"running","id":"qemu/140","kind":"vm"}]}"#,
        )
        .unwrap();

        let resources = load_observed_resources(&ObservationInputs {
            proxmox_json: Some(path.to_str().unwrap()),
            proxmox_list_text: None,
            watchdog_json: None,
        })
        .unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].provider, "proxmox");
        assert_eq!(resources[0].name, "cynic-vm-v2");
        assert_eq!(resources[0].resource_id.as_deref(), Some("qemu/140"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_watchdog_json_resources() {
        let path = temp_file_path("cynic-cli-watchdog-json", ".json");
        fs::write(
            &path,
            r#"{"schema":"cynic.node_watchdog_observation.v1","status":"ok","node":{"name":"cynic-host-cortex","constellation":"cortex"}}"#,
        )
        .unwrap();

        let resources = load_observed_resources(&ObservationInputs {
            proxmox_json: None,
            proxmox_list_text: None,
            watchdog_json: Some(path.to_str().unwrap()),
        })
        .unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].provider, "cynic-node-watchdog");
        assert_eq!(resources[0].name, "cynic-host-cortex");
        assert_eq!(resources[0].status, "running");
        assert_eq!(resources[0].constellation.as_deref(), Some("cortex"));

        let _ = fs::remove_file(path);
    }

    fn temp_file_path(prefix: &str, suffix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}{suffix}"))
    }
}
