use serde_json::Value;

pub(super) fn suite_covers_cloud_prisms(value: &Value) -> Result<(), String> {
    require_array_contains(value, "/cloud_characteristics", "on_demand_self_service")?;
    require_array_contains(value, "/cloud_characteristics", "broad_network_access")?;
    require_array_contains(value, "/cloud_characteristics", "resource_pooling")?;
    require_array_contains(value, "/cloud_characteristics", "rapid_elasticity")?;
    require_array_contains(value, "/cloud_characteristics", "measured_service")?;
    require_array_contains(value, "/service_models", "IaaS")?;
    require_array_contains(value, "/service_models", "PaaS")?;
    require_array_contains(value, "/service_models", "SaaS")?;
    require_array_contains(value, "/deployment_models", "private")?;
    require_array_contains(value, "/deployment_models", "hybrid")?;
    let layers = value
        .get("layers")
        .and_then(Value::as_array)
        .ok_or_else(|| "layers missing".to_string())?;
    let storage = layers
        .iter()
        .find(|layer| layer.get("id").and_then(Value::as_str) == Some("l3_storage_projection"))
        .ok_or_else(|| "l3_storage_projection missing".to_string())?;
    require_array_contains(storage, "/risk_controls", "encryption_at_rest_required")?;
    require_array_contains(storage, "/risk_controls", "encryption_in_transit_required")?;
    require_array_contains(storage, "/risk_controls", "storage_acl_or_rbac_required")?;
    require_array_contains(storage, "/risk_controls", "redundancy_policy_required")?;
    Ok(())
}

pub(super) fn plan_consumes_host_identity_proof(value: &Value) -> Result<(), String> {
    require_array_contains(value, "/observed_gates", "host_identity_observed")?;
    let layers = value
        .get("layers")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing layers array".to_string())?;
    let network_layer = layers
        .iter()
        .find(|layer| layer.get("id").and_then(Value::as_str) == Some("l2_network_projection"))
        .ok_or_else(|| "missing l2_network_projection".to_string())?;
    let missing = network_layer
        .get("missing_entry_gate")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing l2 missing_entry_gate".to_string())?;
    if !missing.is_empty() {
        return Err(format!("l2 still has missing gates: {missing:?}"));
    }
    if network_layer
        .get("can_execute_read_only")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err("l2 read-only probe is not executable after proof".to_string());
    }
    Ok(())
}

pub(super) fn plan_consumes_network_runtime_proof(value: &Value) -> Result<(), String> {
    require_array_contains(value, "/observed_gates", "host_identity_observed")?;
    require_array_contains(value, "/observed_gates", "network_runtime_observed")?;
    let layers = value
        .get("layers")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing layers array".to_string())?;
    let storage_layer = layers
        .iter()
        .find(|layer| layer.get("id").and_then(Value::as_str) == Some("l3_storage_projection"))
        .ok_or_else(|| "missing l3_storage_projection".to_string())?;
    let missing = storage_layer
        .get("missing_entry_gate")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing l3 missing_entry_gate".to_string())?;
    if !missing.is_empty() {
        return Err(format!("l3 still has missing gates: {missing:?}"));
    }
    if storage_layer
        .get("can_execute_read_only")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err("l3 read-only probe is not executable after proof".to_string());
    }
    Ok(())
}

pub(super) fn metrics_consumes_architecture_proof(value: &Value) -> Result<(), String> {
    require_array_contains(
        value,
        "/external_proof_gates",
        "architecture_boundary_verified",
    )?;
    let metrics = value
        .get("metrics")
        .and_then(Value::as_array)
        .ok_or_else(|| "metrics missing".to_string())?;
    let architecture = metrics
        .iter()
        .find(|metric| {
            metric.get("id").and_then(Value::as_str) == Some("architecture_boundary_percent")
        })
        .ok_or_else(|| "architecture_boundary_percent missing".to_string())?;
    if architecture.get("percent").and_then(Value::as_u64) != Some(100) {
        return Err("architecture boundary proof did not produce 100 percent".to_string());
    }
    if architecture.get("status").and_then(Value::as_str) != Some("within_target") {
        return Err("architecture boundary metric is not within_target".to_string());
    }
    Ok(())
}

fn require_array_contains(value: &Value, pointer: &str, needle: &str) -> Result<(), String> {
    let values = value
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{pointer} missing or not array"))?;
    if values.iter().any(|item| item.as_str() == Some(needle)) {
        Ok(())
    } else {
        Err(format!("{pointer} missing {needle}"))
    }
}
