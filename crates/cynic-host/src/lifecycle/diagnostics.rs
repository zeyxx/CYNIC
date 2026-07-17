use serde_json::{Map, Value, json};

pub fn build_activation_evidence_gap_details(
    registry: &Value,
    env: Option<&Map<String, Value>>,
) -> Result<Value, String> {
    let lifecycle = kernel_release(registry)?;
    let role_nodes = activation_role_nodes(registry, lifecycle)?;
    let env = env.cloned().unwrap_or_default();
    let mut gaps = Vec::new();
    let requirements = lifecycle
        .get("activation_requirements")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    for (role, requirement) in requirements {
        let Some(node) = role_nodes.get(&role) else {
            gaps.push(json!({
                "code": format!("{role}:node_not_resolved"),
                "kind": "node",
                "role": role,
                "node": Value::Null,
                "requirement": "resolved_node",
            }));
            continue;
        };
        let node_name = node.get("name").and_then(Value::as_str).unwrap_or("");
        let network = node
            .get("network")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for network_key in requirement
            .get("network")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
        {
            let value = network
                .get(&network_key)
                .and_then(Value::as_str)
                .unwrap_or("");
            if value.is_empty() {
                gaps.push(json!({
                    "code": format!("{node_name}:{network_key}_missing"),
                    "kind": "network",
                    "role": role,
                    "node": node_name,
                    "requirement": network_key,
                    "reason": "missing_registry_value"
                }));
                continue;
            }
            if let Some(env_name) = value.strip_prefix('<').and_then(|v| v.strip_suffix('>'))
                && env
                    .get(env_name)
                    .and_then(value_to_string)
                    .unwrap_or_default()
                    .is_empty()
            {
                gaps.push(json!({
                    "code": format!("{node_name}:{network_key}_env_{env_name}_missing"),
                    "kind": "network",
                    "role": role,
                    "node": node_name,
                    "requirement": network_key,
                    "reason": "missing_env",
                    "env": env_name
                }));
            }
        }

        let service_names = node
            .get("services")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|service| service.get("name").and_then(Value::as_str))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let node_service_evidence = node
            .get("activation_evidence")
            .and_then(|value| value.get("services"))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for service_name in requirement
            .get("services")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
        {
            if !service_names
                .iter()
                .any(|name| name == &service_name.as_str())
            {
                gaps.push(json!({
                    "code": format!("{node_name}:service_{service_name}_missing"),
                    "kind": "service",
                    "role": role,
                    "node": node_name,
                    "requirement": service_name,
                    "reason": "missing_registry_service"
                }));
                continue;
            }
            let active = node_service_evidence
                .get(&service_name)
                .and_then(Value::as_object)
                .and_then(|item| item.get("active"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !active {
                gaps.push(json!({
                    "code": format!("{node_name}:service_{service_name}_inactive"),
                    "kind": "service",
                    "role": role,
                    "node": node_name,
                    "requirement": service_name,
                    "reason": "missing_service_evidence"
                }));
            }
        }

        let runtime_paths = lifecycle
            .get("runtime_paths")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let runtime_path_proofs = lifecycle
            .get("runtime_path_proofs")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let node_runtime_evidence = node
            .get("activation_evidence")
            .and_then(|value| value.get("runtime_paths"))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for path_name in requirement
            .get("filesystem")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
        {
            let verified = node_runtime_evidence
                .get(&path_name)
                .and_then(Value::as_object)
                .and_then(|item| item.get("verified"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !verified {
                gaps.push(json!({
                    "code": format!("{node_name}:runtime_path_{path_name}_unverified"),
                    "kind": "filesystem",
                    "role": role,
                    "node": node_name,
                    "requirement": path_name,
                    "reason": "missing_runtime_path_proof",
                    "path": runtime_paths.get(&path_name).cloned().unwrap_or(Value::Null),
                    "proof": runtime_path_proofs.get(&path_name).cloned().unwrap_or(Value::Object(Map::new()))
                }));
            }
        }
    }

    Ok(Value::Array(gaps))
}

pub fn activation_evidence_gaps(
    registry: &Value,
    env: Option<&Map<String, Value>>,
) -> Result<Value, String> {
    let details = build_activation_evidence_gap_details(registry, env)?;
    let gaps = details
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|gap| {
            gap.get("code")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .collect::<Vec<_>>();
    Ok(json!(gaps))
}

pub fn lifecycle_blockers(
    registry: &Value,
    fleet_probe: Option<&Value>,
    fleet_inventory_probe: Option<&Value>,
    env: Option<&Map<String, Value>>,
) -> Result<Value, String> {
    let lifecycle = kernel_release(registry)?;
    let activation_gap_details = build_activation_evidence_gap_details(registry, env)?;
    let mut blockers = activation_gap_details
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|gap| {
            let mut obj = gap.as_object().cloned().unwrap_or_default();
            obj.insert(
                "category".to_string(),
                Value::String("activation".to_string()),
            );
            Value::Object(obj)
        })
        .collect::<Vec<_>>();

    let required_status = lifecycle
        .get("required_node_status")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !required_status.is_empty() {
        for node in activation_role_nodes(registry, lifecycle)?.values() {
            let node_name = node.get("name").and_then(Value::as_str).unwrap_or("");
            let actual = node
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if actual != required_status {
                blockers.push(json!({
                    "category": "status",
                    "kind": "node_status",
                    "node": node_name,
                    "role": node.get("role").and_then(Value::as_str).unwrap_or(""),
                    "required_status": required_status,
                    "actual_status": actual,
                    "code": format!("{node_name}:status_{actual}_not_{required_status}")
                }));
            }
        }
    }

    let fleet_requirements = lifecycle
        .get("fleet_requirements")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if fleet_requirements
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let provider = fleet_requirements
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("synapse");
        let fleet_probe = fleet_probe
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let fleet_inventory_probe = fleet_inventory_probe
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        if provider == "synapse" {
            let fleet_status = fleet_probe
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("not_run");
            if fleet_status != "ok" {
                let mut blocker = json!({
                    "category": "fleet",
                    "kind": "synapse_probe",
                    "provider": "synapse",
                    "required": "ok",
                    "actual": fleet_status,
                    "error": fleet_probe.get("error").cloned().unwrap_or(Value::Null),
                    "code": format!("fleet_probe:{fleet_status}"),
                    "resolver_organ": "organ-fleet",
                    "resolver_consumer": "organ-forge-hermes-agent"
                });
                if fleet_probe.get("error").and_then(Value::as_str)
                    == Some("synapse_cli_unavailable")
                    && let Some(obj) = blocker.as_object_mut()
                {
                    obj.insert("operator_action".to_string(), Value::String("restore the organ-fleet synapse probe surface on the registry build runner environment".to_string()));
                    obj.insert("command".to_string(), Value::String("python3 organs/fleet/src/entrypoints/cli.py report --json --registry infra/registry.json".to_string()));
                }
                blockers.push(blocker);
            }
            for node_name in fleet_probe
                .get("missing")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
            {
                blockers.push(json!({
                    "category": "fleet",
                    "kind": "synapse_peer",
                    "node": node_name,
                    "required": "present",
                    "actual": "missing",
                    "code": format!("{node_name}:fleet_missing")
                }));
            }
            if fleet_requirements
                .get("require_online")
                .and_then(Value::as_bool)
                .unwrap_or(true)
            {
                for (node_name, peer) in fleet_probe
                    .get("matches")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default()
                {
                    if !peer.get("online").and_then(Value::as_bool).unwrap_or(false) {
                        blockers.push(json!({
                            "category": "fleet",
                            "kind": "synapse_peer",
                            "node": node_name,
                            "required": "online",
                            "actual": "offline",
                            "code": format!("{node_name}:fleet_offline")
                        }));
                    }
                }
            }
        }

        let inventory_status = fleet_inventory_probe
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("not_run");
        if inventory_status != "ok" {
            blockers.push(json!({
                "category": "fleet",
                "kind": "fleet_inventory",
                "provider": "organ-fleet",
                "required": "ok",
                "actual": inventory_status,
                "error": fleet_inventory_probe.get("error").cloned().unwrap_or(Value::Null),
                "code": format!("fleet_inventory:{inventory_status}"),
                "resolver_organ": "organ-fleet",
                "resolver_consumer": "organ-forge-hermes-agent",
                "command": "python3 organs/fleet/src/entrypoints/cli.py report --json --registry infra/registry.json"
            }));
        }
        let summary = fleet_inventory_probe
            .get("summary")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for gap in summary
            .get("provider_evidence_gaps")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            blockers.push(json!({
                "category": "fleet",
                "kind": "fleet_inventory_evidence_gap",
                "provider": "fleet_inventory",
                "code": gap
            }));
        }
        for node_name in summary
            .get("declaration_missing_nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
        {
            blockers.push(json!({
                "category": "fleet",
                "kind": "fleet_inventory_node",
                "node": node_name,
                "required": "declared",
                "actual": "missing",
                "code": format!("{node_name}:fleet_inventory_missing")
            }));
        }
        for node_name in summary
            .get("declaration_unexpected_nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
        {
            blockers.push(json!({
                "category": "fleet",
                "kind": "fleet_inventory_node",
                "node": node_name,
                "required": "declared",
                "actual": "unexpected",
                "code": format!("{node_name}:fleet_inventory_unexpected")
            }));
        }
    }
    Ok(Value::Array(blockers))
}

fn kernel_release(registry: &Value) -> Result<&Value, String> {
    registry
        .get("lifecycle")
        .and_then(|value| value.get("kernel_release"))
        .ok_or_else(|| "registry lifecycle.kernel_release missing".to_string())
}

fn activation_role_nodes(
    registry: &Value,
    lifecycle: &Value,
) -> Result<Map<String, Value>, String> {
    let nodes = registry
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry nodes missing".to_string())?;
    let roles = lifecycle
        .get("activation_requirements")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut result = Map::new();
    for role in roles.keys() {
        if let Some(node) = nodes
            .iter()
            .find(|item| item.get("role").and_then(Value::as_str) == Some(role.as_str()))
        {
            result.insert(role.clone(), node.clone());
        }
    }
    Ok(result)
}

fn value_to_string(value: &Value) -> Option<&str> {
    value.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_fixture() -> Value {
        json!({
            "nodes": [
                {
                    "name": "cynic-vm-anvil",
                    "role": "build_runner",
                    "status": "planned",
                    "network": {"synapse": "<SYNAPSE_ANVIL>"},
                    "services": [{"name": "organ-forge"}, {"name": "cargo-build"}, {"name": "gate-2"}],
                    "activation_evidence": {"runtime_paths": {}, "services": {}}
                },
                {
                    "name": "cynic-vm-core",
                    "role": "kernel_primary",
                    "status": "planned",
                    "network": {"synapse": "<SYNAPSE_CORE>"},
                    "services": [{"name": "cynic-kernel"}, {"name": "cynic-portal"}],
                    "activation_evidence": {"runtime_paths": {}, "services": {}}
                },
                {
                    "name": "cynic-lxc-surrealdb",
                    "role": "storage_primary",
                    "status": "planned",
                    "network": {"synapse": "<SYNAPSE_STORAGE>"},
                    "services": [{"name": "surrealdb"}],
                    "activation_evidence": {"runtime_paths": {}, "services": {}}
                }
            ],
            "lifecycle": {
                "kernel_release": {
                    "required_node_status": "active",
                    "build_runner_role": "build_runner",
                    "deploy_target_role": "kernel_primary",
                    "runtime_paths": {"repo_root": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}"},
                    "runtime_path_proofs": {"repo_root": {"kind": "git_worktree"}},
                    "activation_requirements": {
                        "build_runner": {"network": ["synapse"], "filesystem": ["repo_root"], "services": ["organ-forge", "cargo-build", "gate-2"]},
                        "kernel_primary": {"network": ["synapse"], "filesystem": ["repo_root"], "services": ["cynic-kernel", "cynic-portal"]},
                        "storage_primary": {"network": ["synapse"], "filesystem": [], "services": ["surrealdb"]}
                    },
                    "fleet_requirements": {"required": true, "provider": "synapse", "require_online": true}
                }
            }
        })
    }

    #[test]
    fn activation_gap_details_are_structured() {
        let details =
            build_activation_evidence_gap_details(&registry_fixture(), None).expect("details");
        assert_eq!(details.as_array().map(Vec::len), Some(11));
        assert_eq!(details[0]["kind"], "network");
    }

    #[test]
    fn blockers_include_fleet_probe_failure() {
        let blockers = lifecycle_blockers(
            &registry_fixture(),
            Some(&json!({"status": "error", "error": "synapse_status_failed"})),
            Some(&json!({"enabled": true, "status": "ok"})),
            None,
        )
        .expect("blockers");
        assert!(
            blockers
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item.get("code")
                    == Some(&Value::String("fleet_probe:error".to_string())))
        );
    }
}
