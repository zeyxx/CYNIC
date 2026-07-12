use serde_json::{Value, json};

pub fn validate_workflow_layers(workflow: &Value, orchestrator_organ: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if workflow.get("name").and_then(Value::as_str) != Some("kernel_release") {
        errors.push("lifecycle workflow.name must be kernel_release".to_string());
    }
    if workflow.get("owner").and_then(Value::as_str) != Some(orchestrator_organ) {
        errors.push("lifecycle workflow.owner must match orchestrator_organ".to_string());
    }

    let Some(state_layers) = workflow.get("state_layers").and_then(Value::as_object) else {
        errors.push(
            "lifecycle workflow.state_layers must describe desired/declared/observed/effective/applied/verified"
                .to_string(),
        );
        return errors;
    };
    if state_layers.is_empty() {
        errors.push(
            "lifecycle workflow.state_layers must describe desired/declared/observed/effective/applied/verified"
                .to_string(),
        );
        return errors;
    }

    let expected_layers = [
        ("desired", "human_intent"),
        ("declared", "infra/registry.json"),
        ("observed", "runtime_probes"),
        ("effective", "organ-forge"),
        ("applied", "organ-lifecycle"),
        ("verified", "kernel_readback"),
    ];

    for (layer_name, source) in expected_layers {
        let Some(layer) = state_layers.get(layer_name) else {
            errors.push(format!(
                "lifecycle workflow.state_layers.{layer_name} must be an object"
            ));
            continue;
        };
        let Some(layer_obj) = layer.as_object() else {
            errors.push(format!(
                "lifecycle workflow.state_layers.{layer_name} must be an object"
            ));
            continue;
        };
        if layer_obj.get("source").and_then(Value::as_str) != Some(source) {
            errors.push(format!(
                "lifecycle workflow.state_layers.{layer_name}.source must be {source}"
            ));
        }
        if layer_obj
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
        {
            errors.push(format!(
                "lifecycle workflow.state_layers.{layer_name}.description missing"
            ));
        }
    }

    errors.extend(validate_transition_contract(workflow));
    errors
}

pub fn validate_transition_contract(workflow: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(contract) = workflow.get("transition_contract").and_then(Value::as_object) else {
        return vec!["lifecycle workflow.transition_contract missing".to_string()];
    };
    if contract.is_empty() {
        return vec!["lifecycle workflow.transition_contract missing".to_string()];
    }

    if contract.get("schema").and_then(Value::as_str)
        != Some("cynic.lifecycle_transition_contract.v1")
    {
        errors.push(
            "lifecycle workflow.transition_contract.schema must be cynic.lifecycle_transition_contract.v1"
                .to_string(),
        );
    }

    let expected_layers = [
        "desired",
        "declared",
        "observed",
        "effective",
        "applied",
        "verified",
    ];
    let actual_layers = contract
        .get("layers")
        .and_then(Value::as_array)
        .map(|layers| {
            layers
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if actual_layers != expected_layers {
        errors.push(
            "lifecycle workflow.transition_contract.layers must be desired/declared/observed/effective/applied/verified"
                .to_string(),
        );
    }

    let Some(transitions) = contract.get("transitions").and_then(Value::as_array) else {
        errors.push(
            "lifecycle workflow.transition_contract.transitions must cover each adjacent layer"
                .to_string(),
        );
        return errors;
    };
    if transitions.len() != expected_layers.len() - 1 {
        errors.push(
            "lifecycle workflow.transition_contract.transitions must cover each adjacent layer"
                .to_string(),
        );
        return errors;
    }

    for (transition, pair) in transitions.iter().zip(expected_layers.windows(2)) {
        let [source, target] = [pair[0], pair[1]];
        let Some(transition_obj) = transition.as_object() else {
            errors.push(
                "lifecycle workflow.transition_contract transition must be an object".to_string(),
            );
            continue;
        };
        if transition_obj.get("from").and_then(Value::as_str) != Some(source)
            || transition_obj.get("to").and_then(Value::as_str) != Some(target)
        {
            errors.push(format!(
                "lifecycle workflow.transition_contract must include transition {source}->{target}"
            ));
        }
        if transition_obj
            .get("authority")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
        {
            errors.push(format!(
                "lifecycle transition {source}->{target} authority missing"
            ));
        }
        let evidence_ok = transition_obj
            .get("evidence")
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty());
        if !evidence_ok {
            errors.push(format!(
                "lifecycle transition {source}->{target} evidence missing"
            ));
        }
        if !transition_obj
            .get("read_back_required")
            .is_some_and(Value::is_boolean)
        {
            errors.push(format!(
                "lifecycle transition {source}->{target} read_back_required must be boolean"
            ));
        }
    }

    errors
}

#[allow(clippy::too_many_arguments)]
pub fn build_workflow_state(
    lifecycle: &Value,
    build_node: &Value,
    deploy_node: &Value,
    required_status: Option<&str>,
    activation_gap_details: &[Value],
    activation_gaps: &[String],
    fleet_probe: &Value,
    fleet_inventory_probe: &Value,
    kernel_probe: &Value,
    status_nodes_ready: bool,
    fleet_nodes_ready: bool,
    lifecycle_blockers: &[Value],
) -> Value {
    let workflow = lifecycle.get("workflow").cloned().unwrap_or_else(|| json!({}));
    let state_layers = workflow
        .get("state_layers")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let ready = status_nodes_ready && activation_gaps.is_empty() && fleet_nodes_ready;

    json!({
        "desired": state_layers.get("desired").cloned().unwrap_or_else(|| json!({})),
        "declared": {
            "workflow": workflow,
            "required_node_status": required_status,
            "activation_requirements": lifecycle.get("activation_requirements").cloned().unwrap_or_else(|| json!({})),
            "fleet_requirements": lifecycle.get("fleet_requirements").cloned().unwrap_or_else(|| json!({})),
            "request_profiles": lifecycle.get("request_profiles").cloned().unwrap_or_else(|| json!({})),
            "repo_targets": lifecycle.get("repo_targets").cloned().unwrap_or_else(|| json!([]))
        },
        "observed": {
            "build_target_status": build_node.get("status").and_then(Value::as_str).unwrap_or("unknown"),
            "deploy_target_status": deploy_node.get("status").and_then(Value::as_str).unwrap_or("unknown"),
            "activation_evidence_gaps": activation_gaps,
            "activation_evidence_gap_details": activation_gap_details,
            "fleet_probe": fleet_probe,
            "fleet_inventory_probe": fleet_inventory_probe,
            "kernel_probe": kernel_probe,
            "status_nodes_ready": status_nodes_ready,
            "fleet_nodes_ready": fleet_nodes_ready
        },
        "effective": {
            "lifecycle_nodes_ready": ready,
            "dispatch_allowed": ready,
            "release_allowed": ready,
            "lifecycle_blockers": lifecycle_blockers
        },
        "applied": {
            "source": state_layers.get("applied").cloned().unwrap_or_else(|| json!({})),
            "status": "external_until_runner_result_observed"
        },
        "verified": {
            "source": state_layers.get("verified").cloned().unwrap_or_else(|| json!({})),
            "read_back_required": true,
            "status": "external_until_kernel_readback_observed"
        },
        "transition_contract": workflow.get("transition_contract").cloned().unwrap_or_else(|| json!({}))
    })
}

#[cfg(test)]
mod tests {
    use super::{build_workflow_state, validate_transition_contract, validate_workflow_layers};
    use serde_json::json;

    fn workflow_fixture() -> serde_json::Value {
        json!({
            "name": "kernel_release",
            "owner": "organ-forge",
            "state_layers": {
                "desired": {"source": "human_intent", "description": "intent"},
                "declared": {"source": "infra/registry.json", "description": "registry"},
                "observed": {"source": "runtime_probes", "description": "observed"},
                "effective": {"source": "organ-forge", "description": "effective"},
                "applied": {"source": "organ-lifecycle", "description": "applied"},
                "verified": {"source": "kernel_readback", "description": "verified"}
            },
            "transition_contract": {
                "schema": "cynic.lifecycle_transition_contract.v1",
                "layers": ["desired", "declared", "observed", "effective", "applied", "verified"],
                "transitions": [
                    {"from": "desired", "to": "declared", "authority": "registry", "evidence": ["repo"], "read_back_required": false},
                    {"from": "declared", "to": "observed", "authority": "probe", "evidence": ["probe"], "read_back_required": true},
                    {"from": "observed", "to": "effective", "authority": "forge", "evidence": ["plan"], "read_back_required": true},
                    {"from": "effective", "to": "applied", "authority": "lifecycle", "evidence": ["runner"], "read_back_required": true},
                    {"from": "applied", "to": "verified", "authority": "kernel", "evidence": ["readback"], "read_back_required": true}
                ]
            }
        })
    }

    #[test]
    fn workflow_validation_accepts_expected_contract() {
        let errors = validate_workflow_layers(&workflow_fixture(), "organ-forge");
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn transition_validation_rejects_missing_contract() {
        let errors = validate_transition_contract(&json!({}));
        assert_eq!(errors, vec!["lifecycle workflow.transition_contract missing"]);
    }

    #[test]
    fn build_workflow_state_computes_effective_readiness() {
        let lifecycle = json!({
            "workflow": workflow_fixture(),
            "activation_requirements": {"services": ["cynic-kernel"]},
            "fleet_requirements": {"required": true},
            "request_profiles": {"deploy": ["deploy"]},
            "repo_targets": ["infra/registry.json"]
        });
        let state = build_workflow_state(
            &lifecycle,
            &json!({"status": "active"}),
            &json!({"status": "active"}),
            Some("active"),
            &[],
            &[],
            &json!({"status": "ok"}),
            &json!({"status": "ok"}),
            &json!({"status": "ok"}),
            true,
            true,
            &[],
        );

        assert_eq!(
            state["effective"]["lifecycle_nodes_ready"].as_bool(),
            Some(true)
        );
        assert_eq!(state["effective"]["dispatch_allowed"].as_bool(), Some(true));
        assert_eq!(state["observed"]["build_target_status"].as_str(), Some("active"));
    }
}
