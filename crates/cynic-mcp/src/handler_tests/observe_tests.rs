#[cfg(test)]
mod observe_tests {
    use super::*;

    #[test]
    fn observe_request_body_forwards_k15_ledger_fields() {
        let body = observe_request_body(&ObserveParams {
            tool: "cynic_lifecycle_dispatch".to_string(),
            target: Some("agent_tasks:test".to_string()),
            domain: Some("supply-chain".to_string()),
            status: Some("completed".to_string()),
            context: Some("dispatch persisted and verified".to_string()),
            project: Some("cynic".to_string()),
            agent_id: Some("cortex-test".to_string()),
            session_id: Some("session-test".to_string()),
            tags: Some(vec!["mempool".to_string(), "k15".to_string()]),
            value: Some(serde_json::json!({
                "task_id": "agent_tasks:test",
                "visible": true
            })),
            confidence: Some("verified".to_string()),
            consumer: Some("organ-forge".to_string()),
            action: Some("read_back".to_string()),
            depends_on: Some(vec!["agent_tasks:parent".to_string()]),
            maturity: Some(0.9),
        });

        assert_eq!(body["tool"], "cynic_lifecycle_dispatch");
        assert_eq!(body["consumer"], "organ-forge");
        assert_eq!(body["confidence"], "verified");
        assert_eq!(body["action"], "read_back");
        assert_eq!(body["value"]["task_id"], "agent_tasks:test");
        assert_eq!(body["value"]["visible"], true);
        assert_eq!(body["depends_on"][0], "agent_tasks:parent");
        assert_eq!(body["maturity"], 0.9);
    }
}

