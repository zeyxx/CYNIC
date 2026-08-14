use chrono::{SecondsFormat, Utc};
use serde_json::{Map, Value, json};

pub fn build_lifecycle_service_evidence_patch(
    node_name: &str,
    registry: &Value,
    service_states: Option<&Map<String, Value>>,
) -> Result<Value, String> {
    if node_name.is_empty() {
        return Err("node_name is required".to_string());
    }
    let node = find_node(registry, node_name)?.clone();
    let role = node
        .get("role")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("node {node_name} role missing"))?;
    let required_services = required_services_for_role(registry, role)?;
    if required_services.is_empty() {
        return Err(format!("node {node_name} has no lifecycle services"));
    }
    let declared_services = node
        .get("services")
        .and_then(Value::as_array)
        .map(|services| {
            services
                .iter()
                .filter_map(|service| service.get("name").and_then(Value::as_str))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let checked_at = iso_now();
    let states = service_states.cloned().unwrap_or_default();
    let mut services = Map::new();
    let mut checks = Vec::new();
    let mut ok = true;
    for service_name in &required_services {
        let state = states.get(service_name).and_then(Value::as_object);
        let declared = declared_services.iter().any(|name| name == service_name);
        let active = state
            .and_then(|item| item.get("active"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let checked_service_at = state
            .and_then(|item| item.get("checked_at"))
            .and_then(Value::as_str)
            .unwrap_or(&checked_at);
        let source = state
            .and_then(|item| item.get("source"))
            .and_then(Value::as_str)
            .unwrap_or("lifecycle_service_evidence_patch");
        services.insert(
            service_name.clone(),
            json!({
                "declared": declared,
                "active": active,
                "checked_at": checked_service_at,
                "source": source
            }),
        );
        let check_ok = declared && active;
        ok &= check_ok;
        checks.push(json!({
            "node": node_name,
            "role": role,
            "service": service_name,
            "declared": declared,
            "active": active,
            "ok": check_ok
        }));
    }
    Ok(json!({
        "status": if ok { "ok" } else { "blocked" },
        "lifecycle": "kernel_release",
        "node": node_name,
        "role": role,
        "checks": checks,
        "activation_evidence_patch": {
            "node": node_name,
            "activation_evidence": {
                "services": services
            }
        }
    }))
}

pub fn apply_lifecycle_activation_evidence_patch(
    patch: &Value,
    registry: &Value,
) -> Result<Value, String> {
    let normalized_patch = normalize_patch(patch)?;
    let node_name = normalized_patch
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| "activation evidence patch node missing".to_string())?;
    let nodes = registry
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry nodes missing".to_string())?;
    let node_index = nodes
        .iter()
        .position(|node| node.get("name").and_then(Value::as_str) == Some(node_name))
        .ok_or_else(|| format!("activation evidence patch node is not in registry: {node_name}"))?;
    let node = &nodes[node_index];
    let role = node
        .get("role")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("node {node_name} role missing"))?;
    let required_paths = required_filesystem_for_role(registry, role)?;
    let required_services = required_services_for_role(registry, role)?;
    let declared_services = node
        .get("services")
        .and_then(Value::as_array)
        .map(|services| {
            services
                .iter()
                .filter_map(|service| service.get("name").and_then(Value::as_str))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let activation_evidence = normalized_patch
        .get("activation_evidence")
        .and_then(Value::as_object)
        .ok_or_else(|| "activation_evidence missing".to_string())?;
    let runtime_paths = activation_evidence
        .get("runtime_paths")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let services = activation_evidence
        .get("services")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if runtime_paths.is_empty() && services.is_empty() {
        return Err("activation evidence patch runtime_paths or services missing".to_string());
    }
    let lifecycle = kernel_release(registry)?;
    let runtime_path_values = lifecycle
        .get("runtime_paths")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let runtime_path_proofs = lifecycle
        .get("runtime_path_proofs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut errors = Vec::new();
    for (path_name, path_evidence) in &runtime_paths {
        if !required_paths.iter().any(|item| item == path_name) {
            errors.push(format!(
                "runtime path {path_name} is not required by node role {role}"
            ));
            continue;
        }
        let Some(path_evidence) = path_evidence.as_object() else {
            errors.push(format!(
                "runtime path {path_name} evidence must be an object"
            ));
            continue;
        };
        let expected_value = runtime_path_values.get(path_name).and_then(Value::as_str);
        let expected_proof_kind = runtime_path_proofs
            .get(path_name)
            .and_then(Value::as_object)
            .and_then(|item| item.get("kind"))
            .and_then(Value::as_str);
        if path_evidence.get("value").and_then(Value::as_str) != expected_value {
            errors.push(format!(
                "runtime path {path_name} value does not match registry"
            ));
        }
        if path_evidence.get("proof_kind").and_then(Value::as_str) != expected_proof_kind {
            errors.push(format!(
                "runtime path {path_name} proof_kind does not match registry"
            ));
        }
        for field in ["verified", "marker_exists", "sentinels_verified"] {
            if !path_evidence.get(field).is_some_and(Value::is_boolean) {
                errors.push(format!("runtime path {path_name} {field} must be boolean"));
            }
        }
        if path_evidence
            .get("verified_at")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
        {
            errors.push(format!("runtime path {path_name} verified_at missing"));
        }
        if path_evidence
            .get("verified")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            && (!path_evidence
                .get("marker_exists")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || !path_evidence
                    .get("sentinels_verified")
                    .and_then(Value::as_bool)
                    .unwrap_or(false))
        {
            errors.push(format!(
                "runtime path {path_name} cannot be verified without marker and sentinels"
            ));
        }
    }
    for (service_name, service_evidence) in &services {
        if !required_services.iter().any(|item| item == service_name) {
            errors.push(format!(
                "service {service_name} is not required by node role {role}"
            ));
            continue;
        }
        if !declared_services.iter().any(|name| name == service_name) {
            errors.push(format!(
                "service {service_name} is not declared by node {node_name}"
            ));
            continue;
        }
        let Some(service_evidence) = service_evidence.as_object() else {
            errors.push(format!("service {service_name} evidence must be an object"));
            continue;
        };
        for field in ["declared", "active"] {
            if !service_evidence.get(field).is_some_and(Value::is_boolean) {
                errors.push(format!("service {service_name} {field} must be boolean"));
            }
        }
        if service_evidence
            .get("checked_at")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
        {
            errors.push(format!("service {service_name} checked_at missing"));
        }
        if service_evidence
            .get("active")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            && !service_evidence
                .get("declared")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            errors.push(format!(
                "service {service_name} cannot be active without declared=true"
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    let mut merged = registry.clone();
    let target = merged
        .get_mut("nodes")
        .and_then(Value::as_array_mut)
        .and_then(|nodes| nodes.get_mut(node_index))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "registry nodes missing".to_string())?;
    let activation_evidence = ensure_object(target, "activation_evidence")?;
    let merged_runtime_paths = ensure_object(activation_evidence, "runtime_paths")?;
    for (key, value) in &runtime_paths {
        merged_runtime_paths.insert(key.clone(), value.clone());
    }
    let merged_services = ensure_object(activation_evidence, "services")?;
    for (key, value) in &services {
        merged_services.insert(key.clone(), value.clone());
    }
    Ok(json!({
        "status": "ok",
        "node": node_name,
        "role": role,
        "accepted_runtime_paths": sorted_map_keys(&runtime_paths),
        "accepted_services": sorted_map_keys(&services),
        "registry": merged
    }))
}

pub fn activate_lifecycle_registry_node(
    node_name: &str,
    registry: &Value,
) -> Result<Value, String> {
    if node_name.is_empty() {
        return Err("node_name is required".to_string());
    }
    let lifecycle = kernel_release(registry)?;
    let required_status = lifecycle
        .get("required_node_status")
        .and_then(Value::as_str)
        .ok_or_else(|| "lifecycle required_node_status missing".to_string())?;
    if required_status != "active" {
        return Err("lifecycle required_node_status must be active".to_string());
    }
    let nodes = registry
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry nodes missing".to_string())?;
    let node_index = nodes
        .iter()
        .position(|node| node.get("name").and_then(Value::as_str) == Some(node_name))
        .ok_or_else(|| format!("node {node_name} is not in registry"))?;
    let node = &nodes[node_index];
    let role = node
        .get("role")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("node {node_name} role missing"))?;
    let build_runner_role = lifecycle
        .get("build_runner_role")
        .and_then(Value::as_str)
        .ok_or_else(|| "build_runner_role missing".to_string())?;
    let deploy_target_role = lifecycle
        .get("deploy_target_role")
        .and_then(Value::as_str)
        .ok_or_else(|| "deploy_target_role missing".to_string())?;
    if role != build_runner_role && role != deploy_target_role {
        return Err(format!(
            "node {node_name} is not part of lifecycle kernel_release"
        ));
    }
    let activation_evidence = node
        .get("activation_evidence")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let runtime_paths = activation_evidence
        .get("runtime_paths")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let services = activation_evidence
        .get("services")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let required_paths = required_filesystem_for_role(registry, role)?;
    let required_services = required_services_for_role(registry, role)?;
    let mut gaps = Vec::new();
    for path_name in required_paths {
        let entry = runtime_paths.get(&path_name).and_then(Value::as_object);
        if !entry
            .and_then(|item| item.get("verified"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            gaps.push(format!("{node_name}:{path_name}_missing"));
        }
    }
    for service_name in required_services {
        let entry = services.get(&service_name).and_then(Value::as_object);
        if !entry
            .and_then(|item| item.get("declared"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || !entry
                .and_then(|item| item.get("active"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            gaps.push(format!("{node_name}:{service_name}_inactive"));
        }
    }
    if !gaps.is_empty() {
        return Err(format!(
            "node activation evidence is incomplete ({})",
            gaps.join("; ")
        ));
    }
    let mut merged = registry.clone();
    let target = merged
        .get_mut("nodes")
        .and_then(Value::as_array_mut)
        .and_then(|nodes| nodes.get_mut(node_index))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "registry nodes missing".to_string())?;
    let previous_status = target
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    target.insert(
        "status".to_string(),
        Value::String(required_status.to_string()),
    );
    target.insert("status_updated_at".to_string(), Value::String(iso_now()));
    target.insert(
        "status_source".to_string(),
        Value::String("lifecycle_node_activation".to_string()),
    );
    Ok(json!({
        "status": "ok",
        "node": node_name,
        "role": role,
        "from_status": previous_status,
        "to_status": required_status,
        "registry": merged
    }))
}

fn normalize_patch(patch: &Value) -> Result<Value, String> {
    let Some(obj) = patch.as_object() else {
        return Err("activation evidence patch must be a JSON object".to_string());
    };
    if obj.get("activation_evidence").is_some() {
        return Ok(patch.clone());
    }
    if let Some(wrapped) = obj
        .get("activation_evidence_patch")
        .and_then(Value::as_object)
    {
        return Ok(json!({
            "node": wrapped.get("node").cloned().or_else(|| obj.get("node").cloned()),
            "activation_evidence": wrapped.get("activation_evidence").cloned().unwrap_or(Value::Object(Map::new()))
        }));
    }
    Ok(patch.clone())
}

fn kernel_release(registry: &Value) -> Result<&Value, String> {
    registry
        .get("lifecycle")
        .and_then(|value| value.get("kernel_release"))
        .ok_or_else(|| "registry lifecycle.kernel_release missing".to_string())
}

fn find_node<'a>(registry: &'a Value, node_name: &str) -> Result<&'a Value, String> {
    registry
        .get("nodes")
        .and_then(Value::as_array)
        .and_then(|nodes| {
            nodes
                .iter()
                .find(|node| node.get("name").and_then(Value::as_str) == Some(node_name))
        })
        .ok_or_else(|| format!("node {node_name} is not in registry"))
}

fn required_services_for_role(registry: &Value, role: &str) -> Result<Vec<String>, String> {
    Ok(kernel_release(registry)?
        .get("activation_requirements")
        .and_then(|value| value.get(role))
        .and_then(|value| value.get("services"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default())
}

fn required_filesystem_for_role(registry: &Value, role: &str) -> Result<Vec<String>, String> {
    Ok(kernel_release(registry)?
        .get("activation_requirements")
        .and_then(|value| value.get(role))
        .and_then(|value| value.get("filesystem"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default())
}

fn ensure_object<'a>(
    parent: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    let entry = parent
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    entry
        .as_object_mut()
        .ok_or_else(|| format!("{key} must be an object"))
}

fn sorted_map_keys(value: &Map<String, Value>) -> Vec<String> {
    let mut items = value.keys().cloned().collect::<Vec<_>>();
    items.sort();
    items
}

fn iso_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
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
                    "services": [
                        {"name": "organ-forge"},
                        {"name": "cargo-build"}
                    ],
                    "activation_evidence": {
                        "runtime_paths": {},
                        "services": {}
                    }
                }
            ],
            "lifecycle": {
                "kernel_release": {
                    "required_node_status": "active",
                    "build_runner_role": "build_runner",
                    "deploy_target_role": "kernel_primary",
                    "runtime_paths": {
                        "repo_root": "/var/lib/cynic/repos/${CYNIC_PROJECT_SLUG}"
                    },
                    "runtime_path_proofs": {
                        "repo_root": {"kind": "git_worktree"}
                    },
                    "activation_requirements": {
                        "build_runner": {
                            "filesystem": ["repo_root"],
                            "services": ["organ-forge", "cargo-build"]
                        }
                    }
                }
            }
        })
    }

    #[test]
    fn service_patch_is_registry_derived() {
        let payload =
            build_lifecycle_service_evidence_patch("cynic-vm-anvil", &registry_fixture(), None)
                .expect("service patch");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["checks"].as_array().map(Vec::len), Some(2));
    }

    #[test]
    fn apply_patch_merges_service_proof() {
        let patch = json!({
            "node": "cynic-vm-anvil",
            "activation_evidence": {
                "services": {
                    "organ-forge": {
                        "declared": true,
                        "active": true,
                        "checked_at": "2026-06-15T00:00:00Z"
                    }
                }
            }
        });
        let result = apply_lifecycle_activation_evidence_patch(&patch, &registry_fixture())
            .expect("apply patch");
        assert_eq!(result["status"], "ok");
        assert_eq!(result["accepted_services"], json!(["organ-forge"]));
    }

    #[test]
    fn activate_node_requires_complete_evidence() {
        let err = activate_lifecycle_registry_node("cynic-vm-anvil", &registry_fixture())
            .expect_err("missing evidence should block");
        assert!(err.contains("node activation evidence is incomplete"));
    }
}
