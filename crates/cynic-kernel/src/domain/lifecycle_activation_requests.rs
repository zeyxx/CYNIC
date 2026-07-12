mod correlation;
mod scope;
mod summary;
mod validation;

pub use correlation::{
    activation_correlation_id_from_content, activation_request_key_from_content,
};
pub use scope::{activation_request_priority, activation_request_targets};
pub use summary::{
    activation_request_identity, activation_request_summary, pending_activation_task_by_key,
};
pub use validation::validate_activation_request_payload;

pub const ACTIVATION_REQUEST_CONSUMER: &str = "organ-forge-hermes-agent";
pub const ACTIVATION_SCHEMA_VERSION: &str = "activation.v1";

#[cfg(test)]
mod tests {
    use super::{
        ACTIVATION_REQUEST_CONSUMER, ACTIVATION_SCHEMA_VERSION, activation_correlation_id_from_content,
        activation_request_priority, activation_request_targets, validate_activation_request_payload,
    };
    use serde_json::{Value, json};

    fn registry_fixture() -> Value {
        json!({
            "lifecycle": {
                "kernel_release": {
                    "orchestrator_organ": "organ-forge",
                    "task_kind": "hermes",
                    "task_domain": "organ-forge",
                    "build_runner_role": "build_runner",
                    "deploy_target_role": "kernel_primary",
                    "deploy_target_flag": "deploy_target",
                    "required_node_status": "active",
                    "activation_requirements": {"build_runner": {"network": ["synapse"]}},
                    "fleet_requirements": {"required": true, "provider": "synapse"},
                    "runtime_paths": {"repo_root": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}"},
                    "runtime_path_proofs": {"repo_root": {"kind": "git_worktree"}},
                    "activation_evidence_contract": {"patch_field": "activation_evidence_patch"}
                }
            },
            "nodes": [
                {"name": "cynic-vm-build", "role": "build_runner"},
                {"name": "cynic-vm-core", "role": "kernel_primary", "deploy_target": true}
            ]
        })
    }

    fn payload_fixture() -> Value {
        let action = json!({"action": "materialize_runtime_path", "node": "cynic-vm-build"});
        let (targets, coord_targets) = activation_request_targets(&action);
        let mut content = json!({
            "lifecycle": "kernel_release",
            "activation_schema_version": ACTIVATION_SCHEMA_VERSION,
            "objective": "activate materialize_runtime_path for cynic-vm-build",
            "orchestrator_organ": "organ-forge",
            "domain": "organ-forge",
            "build_target": "cynic-vm-build",
            "build_runner_role": "build_runner",
            "deploy_target": "cynic-vm-core",
            "deploy_target_role": "kernel_primary",
            "required_node_status": "active",
            "activation_requirements": {"build_runner": {"network": ["synapse"]}},
            "fleet_requirements": {"required": true, "provider": "synapse"},
            "runtime_paths": {"repo_root": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}"},
            "runtime_path_proofs": {"repo_root": {"kind": "git_worktree"}},
            "activation_evidence_contract": {"patch_field": "activation_evidence_patch"},
            "targets": targets,
            "coord_targets": coord_targets,
            "deploy": false,
            "action": action,
            "operator_interface_proposal": {"kind": "proposal"}
        });
        let correlation_id = activation_correlation_id_from_content(&content).expect("correlation");
        content["correlation_id"] = json!(correlation_id);
        json!({
            "kind": "hermes",
            "domain": "organ-forge",
            "agent_id": "organ-forge-cli",
            "consumer": ACTIVATION_REQUEST_CONSUMER,
            "priority": activation_request_priority(content.get("action").expect("action")),
            "content": serde_json::to_string(&content).expect("content")
        })
    }

    #[test]
    fn targets_route_registry_only_actions_to_registry_scope() {
        let (targets, coord_targets) =
            activation_request_targets(&json!({"action": "materialize_runtime_path"}));
        assert_eq!(targets, vec!["infra/registry.json"]);
        assert_eq!(coord_targets, vec!["infra/registry.json"]);
    }

    #[test]
    fn validation_accepts_registry_derived_payload() {
        let validated =
            validate_activation_request_payload(&payload_fixture(), &registry_fixture()).expect("valid");
        assert_eq!(validated["status"].as_str(), Some("ok"));
    }

    #[test]
    fn validation_rejects_fake_global_scope() {
        let registry = registry_fixture();
        let mut payload = payload_fixture();
        let mut content: Value = serde_json::from_str(
            payload.get("content").and_then(Value::as_str).expect("content"),
        )
        .expect("parse");
        content["targets"] = json!(["organs/forge", "infra/registry.json"]);
        content["coord_targets"] = json!(["organs/forge", "infra/registry.json"]);
        payload["content"] = json!(serde_json::to_string(&content).expect("content"));

        let error = validate_activation_request_payload(&payload, &registry).expect_err("error");
        assert!(error.contains("activation action scope"), "{error}");
    }

    #[test]
    fn validation_rejects_priority_drift() {
        let registry = registry_fixture();
        let mut payload = payload_fixture();
        payload["priority"] = json!(1);
        let error = validate_activation_request_payload(&payload, &registry).expect_err("error");
        assert!(error.contains("payload.priority"), "{error}");
    }

    #[test]
    fn validation_rejects_correlation_drift() {
        let registry = registry_fixture();
        let mut payload = payload_fixture();
        let mut content: Value = serde_json::from_str(
            payload.get("content").and_then(Value::as_str).expect("content"),
        )
        .expect("parse");
        content["correlation_id"] = json!("activation:0000000000000000");
        payload["content"] = json!(serde_json::to_string(&content).expect("content"));
        let error = validate_activation_request_payload(&payload, &registry).expect_err("error");
        assert!(error.contains("content.correlation_id"), "{error}");
    }
}
