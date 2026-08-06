use cynic_inference::MultiAgentService;

#[tokio::test]
async fn test_live_debate_round_on_local_inference() {
    let service = MultiAgentService::new(None);
    let session = service
        .run_debate_round(
            "session-live-001",
            "Découplage strict des adaptateurs d'inférence du superviseur systemd dans CYNIC",
            "lfm25-2b-q8",
            "http://127.0.0.1:8080/v1",
            "lfm25-2b-q8",
            "http://127.0.0.1:8080/v1",
        )
        .await;

    assert_eq!(session.session_id, "session-live-001");
    assert_eq!(session.turns.len(), 2);
    assert!(session.consensus_reached);
    assert!(session.final_consensus_text.is_some());
    assert_eq!(session.schema, "cynic.inference.multi_agent_debate.v1");
}
