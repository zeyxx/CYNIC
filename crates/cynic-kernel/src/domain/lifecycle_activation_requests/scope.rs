use serde_json::Value;

pub fn activation_request_targets(action: &Value) -> (Vec<String>, Vec<String>) {
    let action_name = action.get("action").and_then(Value::as_str).unwrap_or("");
    let targets = match action_name {
        "provide_placeholder_env_evidence"
        | "verify_runtime_path_proof"
        | "record_service_activation_evidence"
        | "activate_registry_node"
        | "materialize_runtime_path" => vec!["infra/registry.json".to_string()],
        "refresh_fleet_inventory" | "restore_fleet_probe" => vec!["organs/fleet".to_string()],
        "provide_runtime_observation" | "join_or_wake_synapse_peer" => {
            vec!["organs/fleet".to_string(), "crates/cynic-node".to_string()]
        }
        "restore_canonical_dns_projection"
        | "install_network_actuator_bundle"
        | "activate_synapse_route"
        | "activate_wireguard_mesh" => {
            vec!["organs/fleet".to_string(), "crates/cynic-node".to_string()]
        }
        _ => vec!["organs/forge".to_string(), "infra/registry.json".to_string()],
    };
    (dedupe(targets.clone()), dedupe(targets))
}

pub fn activation_request_priority(action: &Value) -> i64 {
    match action.get("action").and_then(Value::as_str).unwrap_or("") {
        "provide_placeholder_env_evidence" | "record_service_activation_evidence" => 99,
        "restore_fleet_probe" => 98,
        "refresh_fleet_inventory"
        | "provide_runtime_observation"
        | "restore_canonical_dns_projection"
        | "install_network_actuator_bundle"
        | "activate_synapse_route"
        | "activate_wireguard_mesh" => 97,
        "materialize_runtime_path" => 96,
        "verify_runtime_path_proof" | "activate_registry_node" => 95,
        _ => 90,
    }
}

pub(super) fn value_string_vec(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn dedupe(items: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for item in items {
        if !item.is_empty() && !unique.contains(&item) {
            unique.push(item);
        }
    }
    unique
}
