#[cfg(test)]
mod lifecycle_dispatch_tests {
    use super::*;

    #[test]
    fn lifecycle_payload_body_defaults_to_forge_dispatch_policy() {
        let payload = serde_json::json!({
            "kind": "hermes",
            "domain": "gate-2-request",
            "agent_id": "organ-forge-request",
            "content": "{\"intent\":\"build-and-test\"}"
        });

        let body = lifecycle_payload_body_with_defaults(&payload);

        assert_eq!(body["priority"], 99);
        assert_eq!(body["consumer"], "cynic-runtime");
        assert_eq!(
            body["escalation_policy"],
            "canonical_build_runner_validation_only_no_cortex_build"
        );
        assert_eq!(body["domain"], "gate-2-request");
    }

    #[test]
    fn lifecycle_payload_body_preserves_explicit_policy_fields() {
        let payload = serde_json::json!({
            "kind": "hermes",
            "domain": "deploy-request",
            "priority": 77,
            "consumer": "custom-forge",
            "escalation_policy": "custom",
            "content": "{\"intent\":\"deploy\"}"
        });

        let body = lifecycle_payload_body_with_defaults(&payload);

        assert_eq!(body["priority"], 77);
        assert_eq!(body["consumer"], "custom-forge");
        assert_eq!(body["escalation_policy"], "custom");
    }

    #[test]
    fn generic_dispatch_result_accepts_pending_read_back_visibility() {
        let response = serde_json::json!({"task_id": "agent_tasks:pending"});
        let pending = serde_json::json!({
            "tasks": [{"id": "agent_tasks:pending"}],
            "count": 1
        });
        let completed = serde_json::json!({"tasks": [], "count": 0});

        let result = verified_agent_task_dispatch_result(
            &response,
            "hermes",
            "mcp-runtime",
            &pending,
            &completed,
        )
        .expect("pending task id should prove dispatch visibility");

        assert_eq!(result["status"], "dispatched");
        assert_eq!(result["task_id"], "agent_tasks:pending");
        assert_eq!(result["visible_dispatched_ids"][0], "agent_tasks:pending");
    }

    #[test]
    fn generic_dispatch_result_rejects_missing_read_back_visibility() {
        let response = serde_json::json!({"task_id": "agent_tasks:missing"});
        let pending = serde_json::json!({"tasks": [], "count": 0});
        let completed = serde_json::json!({"tasks": [], "count": 0});

        let result = verified_agent_task_dispatch_result(
            &response,
            "hermes",
            "mcp-runtime",
            &pending,
            &completed,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read-back incomplete"));
    }

    #[test]
    fn task_completion_result_accepts_completed_read_back_visibility() {
        let response = serde_json::json!({"task_id": "agent_tasks:done", "status": "completed"});
        let completed = serde_json::json!({
            "tasks": [{"id": "agent_tasks:done"}],
            "count": 1
        });

        let result = verified_agent_task_completion_result(
            &response,
            "agent_tasks:done",
            "hermes",
            Some("gate-2-request"),
            &completed,
        )
        .expect("completed task id should prove result visibility");

        assert_eq!(result["status"], "completed");
        assert_eq!(result["task_id"], "agent_tasks:done");
        assert_eq!(result["read_back_verified"], true);
        assert_eq!(result["visible_completed_ids"][0], "agent_tasks:done");
    }

    #[test]
    fn task_completion_result_rejects_missing_read_back_visibility() {
        let response = serde_json::json!({"task_id": "agent_tasks:missing", "status": "completed"});
        let completed = serde_json::json!({"tasks": [], "count": 0});

        let result = verified_agent_task_completion_result(
            &response,
            "agent_tasks:missing",
            "hermes",
            Some("gate-2-request"),
            &completed,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read-back incomplete"));
    }

    #[test]
    fn lifecycle_payloads_from_plan_rejects_empty_payloads() {
        let payload_plan = serde_json::json!({
            "status": "ok",
            "payloads": []
        });

        let result = lifecycle_payloads_from_plan(&payload_plan);

        assert!(result.is_err());
    }

    #[test]
    fn lifecycle_payload_dry_run_result_marks_non_mutating_plan() {
        let payload_plan = serde_json::json!({
            "status": "ok",
            "payloads": [{"kind": "hermes", "domain": "gate-2-request"}]
        });

        let result = lifecycle_payload_dry_run_result(payload_plan);

        assert_eq!(result["dry_run"], true);
        assert_eq!(result["mutates_mempool"], false);
    }

    #[test]
    fn lifecycle_payload_intent_extracts_inner_content_intent() {
        let payload = serde_json::json!({
            "content": "{\"lifecycle\":\"kernel_release\",\"intent\":\"integration-test\"}"
        });

        assert_eq!(
            lifecycle_payload_intent(&payload),
            Some(serde_json::json!("integration-test"))
        );
    }

    #[test]
    fn lifecycle_payload_domain_uses_first_payload_or_profile_fallback() {
        let payloads = vec![serde_json::json!({"domain": "gate-2-request"})];

        assert_eq!(
            lifecycle_payload_domain(&payloads, "gate-2"),
            "gate-2-request"
        );
        assert_eq!(lifecycle_payload_domain(&[], "gate-2"), "gate-2");
    }

    #[test]
    fn visible_dispatched_task_ids_accepts_pending_or_completed_visibility() {
        let dispatched = vec![
            serde_json::json!({"task_id": "agent_tasks:pending"}),
            serde_json::json!({"task_id": "agent_tasks:completed"}),
            serde_json::json!({"task_id": "agent_tasks:missing"}),
        ];
        let pending = serde_json::json!({
            "tasks": [{"id": "agent_tasks:pending"}],
            "count": 1
        });
        let completed = serde_json::json!({
            "tasks": [{"id": "agent_tasks:completed"}],
            "count": 1
        });

        let (dispatched_ids, visible_ids) =
            visible_dispatched_task_ids(&dispatched, &pending, &completed);

        assert_eq!(dispatched_ids.len(), 3);
        assert_eq!(
            visible_ids,
            vec![
                "agent_tasks:pending".to_string(),
                "agent_tasks:completed".to_string()
            ]
        );
    }

    #[test]
    fn missing_dispatched_task_ids_detects_partial_read_back() {
        let dispatched_ids = vec![
            "agent_tasks:pending".to_string(),
            "agent_tasks:completed".to_string(),
            "agent_tasks:missing".to_string(),
        ];
        let visible_ids = vec![
            "agent_tasks:pending".to_string(),
            "agent_tasks:completed".to_string(),
        ];

        let missing = missing_dispatched_task_ids(&dispatched_ids, &visible_ids);

        assert_eq!(missing, vec!["agent_tasks:missing".to_string()]);
    }
}

