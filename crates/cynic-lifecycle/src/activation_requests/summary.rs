use serde_json::{Map, Value, json};

use super::correlation::activation_correlation_id_from_content;

const OPTIONAL_ACTION_FIELDS: &[&str] = &[
    "operator_action",
    "bundle_command",
    "install_plan_command",
    "install_commands",
    "manual_root_install_command",
    "activation_plan_command",
    "activation_status_command",
    "postflight_commands",
    "validation_command",
    "missing",
    "helper_path",
    "sudoers_rule",
    "bundle_complete",
    "next_actions",
    "preferred_transport",
    "fallback_transport",
    "transport_command_env",
    "transport_command_default",
    "remote_exec_request",
    "mcp_reload_command",
    "legacy_mcp_reload_command",
    "emergency_repair_command",
    "dispatch_profile",
    "dispatch_tool",
    "request_intents",
    "request_make_target",
    "proof_requirements",
    "debounce_seconds",
    "repair_local_debug_binary",
    "canonical_binary",
    "stale_source",
    "recommended_build_profile",
    "transport_failure_mode",
    "transport_timeout_seconds",
    "response_error_message",
    "response_error_code",
    "required_bridge",
    "transport_reason",
    "transport_missing",
    "declared_role",
    "declared_transport",
];

pub fn activation_request_identity(payload: &Value) -> Result<Value, String> {
    let content = payload_content(payload)?;
    Ok(json!({
        "key": super::correlation::activation_request_key_from_content(&content)?,
        "correlation_id": activation_correlation_id_from_content(&content)?,
        "action": content
            .get("action")
            .and_then(|v| v.get("action"))
            .cloned()
            .unwrap_or(Value::Null)
    }))
}

pub fn activation_request_summary(payload: &Value) -> Result<Value, String> {
    let content = payload_content(payload)?;
    let action = content
        .get("action")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut summary = Map::from_iter([
        (
            "correlation_id".to_string(),
            json!(activation_correlation_id_from_content(&content)?),
        ),
        (
            "action".to_string(),
            action.get("action").cloned().unwrap_or(Value::Null),
        ),
        (
            "node".to_string(),
            action.get("node").cloned().unwrap_or(Value::Null),
        ),
        (
            "hostname".to_string(),
            action.get("hostname").cloned().unwrap_or(Value::Null),
        ),
        (
            "role".to_string(),
            action.get("role").cloned().unwrap_or(Value::Null),
        ),
        (
            "code".to_string(),
            action.get("code").cloned().unwrap_or(Value::Null),
        ),
        (
            "targets".to_string(),
            content.get("targets").cloned().unwrap_or_else(|| json!([])),
        ),
        (
            "coord_targets".to_string(),
            content
                .get("coord_targets")
                .cloned()
                .unwrap_or_else(|| json!([])),
        ),
        (
            "consumer".to_string(),
            payload.get("consumer").cloned().unwrap_or(Value::Null),
        ),
        (
            "priority".to_string(),
            payload.get("priority").cloned().unwrap_or(Value::Null),
        ),
        (
            "resolver_organ".to_string(),
            action.get("resolver_organ").cloned().unwrap_or(Value::Null),
        ),
        (
            "resolver_consumer".to_string(),
            action
                .get("resolver_consumer")
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "command".to_string(),
            action.get("command").cloned().unwrap_or(Value::Null),
        ),
    ]);

    for key in OPTIONAL_ACTION_FIELDS {
        if let Some(value) = action.get(*key) {
            summary.insert((*key).to_string(), value.clone());
        }
    }
    Ok(Value::Object(summary))
}

pub fn pending_activation_task_by_key(tasks: &Value) -> Result<Value, String> {
    let mut indexed = Map::new();
    for task in tasks.as_array().cloned().unwrap_or_default() {
        let Some(task_obj) = task.as_object() else {
            continue;
        };
        let Ok(content) = task_obj
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| "payload.content is not valid JSON".to_string())
            .and_then(|raw| {
                serde_json::from_str::<Value>(raw)
                    .map_err(|_| "payload.content is not valid JSON".to_string())
            })
        else {
            continue;
        };
        let key = super::correlation::activation_request_key_from_content(&content)?;
        indexed
            .entry(key)
            .or_insert(Value::Object(task_obj.clone()));
    }
    Ok(Value::Object(indexed))
}

fn payload_content(payload: &Value) -> Result<Value, String> {
    match payload.get("content") {
        Some(Value::String(raw)) => serde_json::from_str::<Value>(raw)
            .map_err(|_| "payload.content is not valid JSON".to_string()),
        Some(Value::Object(obj)) => Ok(Value::Object(obj.clone())),
        _ => Err("payload.content is not valid JSON".to_string()),
    }
}
