use serde_json::{Value, json};

use super::correlation::activation_correlation_id_from_content;
use super::scope::{activation_request_priority, activation_request_targets, value_string_vec};
use super::{ACTIVATION_REQUEST_CONSUMER, ACTIVATION_SCHEMA_VERSION};

pub fn validate_activation_request_payload(
    payload: &Value,
    registry: &Value,
) -> Result<Value, String> {
    let lifecycle = registry
        .get("lifecycle")
        .and_then(|v| v.get("kernel_release"))
        .ok_or_else(|| "missing lifecycle.kernel_release".to_string())?;

    let mut errors = Vec::new();

    if payload.get("kind").and_then(Value::as_str)
        != lifecycle.get("task_kind").and_then(Value::as_str)
    {
        errors.push("payload.kind does not match lifecycle task_kind".to_string());
    }
    if payload.get("domain").and_then(Value::as_str) != Some("organ-forge") {
        errors.push("payload.domain does not match activation request domain".to_string());
    }
    if payload.get("agent_id").and_then(Value::as_str) != Some("organ-forge-cli") {
        errors.push("payload.agent_id does not match activation request agent".to_string());
    }
    if payload.get("consumer").and_then(Value::as_str) != Some(ACTIVATION_REQUEST_CONSUMER) {
        errors.push("payload.consumer does not match activation request consumer".to_string());
    }

    let content = payload
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| "payload.content is not valid JSON".to_string())
        .and_then(|raw| {
            serde_json::from_str::<Value>(raw)
                .map_err(|_| "payload.content is not valid JSON".to_string())
        })?;

    let nodes = registry
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let build_node = nodes.iter().find(|node| {
        node.get("role").and_then(Value::as_str)
            == lifecycle.get("build_runner_role").and_then(Value::as_str)
    });
    let deploy_flag = lifecycle
        .get("deploy_target_flag")
        .and_then(Value::as_str)
        .unwrap_or("deploy_target");
    let deploy_role = lifecycle.get("deploy_target_role").and_then(Value::as_str);
    let deploy_node = nodes.iter().find(|node| {
        node.get("role").and_then(Value::as_str) == deploy_role
            && node
                .get(deploy_flag)
                .and_then(Value::as_bool)
                .unwrap_or(false)
    });

    let expected_pairs: [(&str, Value); 11] = [
        ("lifecycle", json!("kernel_release")),
        (
            "activation_schema_version",
            json!(ACTIVATION_SCHEMA_VERSION),
        ),
        (
            "orchestrator_organ",
            lifecycle
                .get("orchestrator_organ")
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "domain",
            lifecycle.get("task_domain").cloned().unwrap_or(Value::Null),
        ),
        (
            "build_target",
            build_node
                .and_then(|node| node.get("name"))
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "build_runner_role",
            lifecycle
                .get("build_runner_role")
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "deploy_target",
            deploy_node
                .and_then(|node| node.get("name"))
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "deploy_target_role",
            lifecycle
                .get("deploy_target_role")
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "required_node_status",
            lifecycle
                .get("required_node_status")
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "activation_requirements",
            lifecycle
                .get("activation_requirements")
                .cloned()
                .unwrap_or_else(|| json!({})),
        ),
        (
            "fleet_requirements",
            lifecycle
                .get("fleet_requirements")
                .cloned()
                .unwrap_or_else(|| json!({})),
        ),
    ];
    for (key, expected) in expected_pairs {
        if content.get(key).cloned().unwrap_or(Value::Null) != expected {
            errors.push(format!("content.{key} does not match lifecycle registry"));
        }
    }
    for key in [
        "runtime_paths",
        "runtime_path_proofs",
        "activation_evidence_contract",
    ] {
        let expected = lifecycle.get(key).cloned().unwrap_or_else(|| json!({}));
        if content.get(key).cloned().unwrap_or(Value::Null) != expected {
            errors.push(format!("content.{key} does not match lifecycle registry"));
        }
    }
    if content.get("deploy").and_then(Value::as_bool) != Some(false) {
        errors.push("content.deploy does not match lifecycle registry".to_string());
    }

    let Some(action) = content.get("action") else {
        errors.push("content.action must be a structured activation action".to_string());
        return finalize_validation_errors(errors);
    };
    if action
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty()
    {
        errors.push("content.action must be a structured activation action".to_string());
    } else {
        let (expected_targets, expected_coord_targets) = activation_request_targets(action);
        if value_string_vec(content.get("targets")) != expected_targets {
            errors.push("content.targets does not match activation action scope".to_string());
        }
        if value_string_vec(content.get("coord_targets")) != expected_coord_targets {
            errors.push("content.coord_targets does not match activation action scope".to_string());
        }
        if payload.get("priority").and_then(Value::as_i64)
            != Some(activation_request_priority(action))
        {
            errors.push("payload.priority does not match activation action priority".to_string());
        }
        match activation_correlation_id_from_content(&content) {
            Ok(correlation_id) => {
                if content.get("correlation_id").and_then(Value::as_str)
                    != Some(correlation_id.as_str())
                {
                    errors.push(
                        "content.correlation_id does not match activation action scope".to_string(),
                    );
                }
            }
            Err(err) => errors.push(err),
        }
    }

    if content
        .get("objective")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty()
    {
        errors.push("content.objective missing".to_string());
    }
    if !content
        .get("operator_interface_proposal")
        .is_some_and(Value::is_object)
    {
        errors.push("content.operator_interface_proposal missing".to_string());
    }

    finalize_validation_errors(errors)?;
    Ok(json!({
        "status": "ok",
        "lifecycle": "kernel_release",
        "action": action.get("action").cloned().unwrap_or(Value::Null),
        "objective": content.get("objective").cloned().unwrap_or(Value::Null)
    }))
}

fn finalize_validation_errors(errors: Vec<String>) -> Result<Value, String> {
    if errors.is_empty() {
        Ok(Value::Null)
    } else {
        Err(errors.join("; "))
    }
}
