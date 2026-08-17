use cynic_protocol::{FleetDiscoveryNode, FleetKernelDiscoveryReport, derive_kernel_candidates};
use serde_json::Value;
use std::path::Path;

pub async fn discover_kernel_placement(
    registry_path: &Path,
) -> Result<FleetKernelDiscoveryReport, String> {
    let raw = std::fs::read_to_string(registry_path).map_err(|error| {
        format!(
            "failed to read registry {}: {error}",
            registry_path.display()
        )
    })?;
    let registry: Value = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "failed to parse registry {} as JSON: {error}",
            registry_path.display()
        )
    })?;
    let inventory = crate::fleet_inventory::build_fleet_inventory_details(registry_path).await?;
    let now_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let inventory_binding_violations = cynic_protocol::fleet_inventory_read_back_violations(
        &registry,
        inventory.registry_digest.as_deref(),
        inventory.observed_at_unix_ms,
        now_unix_ms,
    );
    let inventory_bound = inventory_binding_violations.is_empty();
    let observations = inventory
        .nodes
        .iter()
        .map(|node| {
            let declared = declared_node(&registry, &node.node_id, node.meta.as_ref());
            let resource_id = declared
                .and_then(|entry| entry.get("proxmox_id").and_then(Value::as_str))
                .map(ToString::to_string)
                .or_else(|| resource_id(&node.node_id));
            let resource_kind = resource_id
                .as_deref()
                .and_then(|id| id.split_once('/').map(|(kind, _)| kind.to_string()));
            let mut evidence_refs = Vec::new();
            if node.observed && inventory_bound {
                evidence_refs.push("provider_inventory_read_back".to_string());
            }
            if node.provider == "registry" {
                evidence_refs.push("registry_declared_node".to_string());
            }
            FleetDiscoveryNode {
                node_id: node.node_id.clone(),
                provider: node.provider.clone(),
                provider_node: node.provider_node.clone().or_else(|| {
                    node.meta
                        .as_ref()
                        .and_then(|meta| meta.get("provider_node").cloned())
                }),
                resource_id,
                resource_kind,
                power: node.status.clone(),
                address: node.ip.clone(),
                declared_role: declared
                    .and_then(|entry| entry.get("role").and_then(Value::as_str))
                    .map(ToString::to_string)
                    .or_else(|| {
                        node.meta
                            .as_ref()
                            .and_then(|meta| meta.get("role"))
                            .cloned()
                    }),
                provider_observed: node.observed && node.provider != "registry" && inventory_bound,
                evidence_refs,
                unknowns: if !inventory_bound {
                    inventory_binding_violations.clone()
                } else if node.observed {
                    Vec::new()
                } else {
                    vec!["provider_inventory_unobserved".to_string()]
                },
            }
        })
        .collect::<Vec<_>>();
    let mut report = derive_kernel_candidates(&registry, &observations);
    report.observed_at_unix_ms = Some(inventory.observed_at_unix_ms);
    Ok(report)
}

fn resource_id(node_id: &str) -> Option<String> {
    node_id
        .split_once('/')
        .filter(|(_, id)| !id.is_empty())
        .map(|_| node_id.to_string())
}

fn declared_node<'a>(
    registry: &'a Value,
    node_id: &str,
    meta: Option<&std::collections::BTreeMap<String, String>>,
) -> Option<&'a Value> {
    registry.get("nodes")?.as_array()?.iter().find(|node| {
        node.get("id").and_then(Value::as_str) == Some(node_id)
            || node.get("name").and_then(Value::as_str) == Some(node_id)
            || node.get("proxmox_id").and_then(Value::as_str) == Some(node_id)
            || meta
                .and_then(|values| values.get("role"))
                .zip(node.get("role").and_then(Value::as_str))
                .is_some_and(|(left, right)| left == right && node.get("id").is_some())
    })
}
