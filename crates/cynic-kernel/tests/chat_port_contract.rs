//! ChatPort trait contract tests — any ChatPort implementation must pass these.

use cynic_kernel::domain::chat::*;
use cynic_kernel::domain::inference::BackendStatus;

async fn chat_port_contract(port: &dyn ChatPort) {
    // 1. Name must be non-empty
    assert!(!port.name().is_empty(), "ChatPort must have non-empty name");

    // 2. Health must return a valid BackendStatus
    let health = port.health().await;
    match health {
        BackendStatus::Unknown
        | BackendStatus::Healthy
        | BackendStatus::Degraded { .. }
        | BackendStatus::Critical
        | BackendStatus::Recovering => {} // all valid
    }

    // 3. Chat must return Ok or a well-formed error (Scoring profile = most constrained)
    let result = port
        .chat(
            "You are a test.",
            "Say hello.",
            InferenceProfile::SCORING,
            None,
        )
        .await;
    match result {
        Ok(resp) => assert!(
            !resp.text.is_empty(),
            "Successful chat should return non-empty text"
        ),
        Err(ChatError::Unreachable(_)) => {}
        Err(ChatError::Timeout { .. }) => {}
        Err(ChatError::RateLimited(_)) => {}
        Err(ChatError::Protocol(_)) => {}
    }
}

#[tokio::test]
async fn mock_chat_passes_contract() {
    let mock = MockChatBackend::new("test-mock", "Hello!");
    chat_port_contract(&mock).await;
}
