use cynic_intel::solana::{SolanaPayRequest, ZeroClawSolanaPlugin};
use cynic_protocol::agent_recursion::AgentBudget;
use cynic_protocol::agentic_tool_call::AgenticToolCallStatus;

#[tokio::test]
async fn plugin_generates_solana_pay_and_qr() {
    let plugin = ZeroClawSolanaPlugin::new();
    let pay_req = SolanaPayRequest::new("7Xw7K1kPzSjW7Y2qM3nK4pL5rQ8sT9uV8wX1yZ2a3b4c")
        .with_amount(15.5)
        .with_label("Superteam Brasil Shop")
        .with_message("Bounty Invoice #101")
        .with_memo("INV-101");

    let budget = AgentBudget {
        context_tokens: 1000,
        actions: 5,
        children: 2,
        wall_time_ms: 10000,
        cpu_ms: None,
    };

    let output = plugin.generate_solana_pay(pay_req, None, budget).await;
    assert!(
        output
            .url
            .starts_with("solana:7Xw7K1kPzSjW7Y2qM3nK4pL5rQ8sT9uV8wX1yZ2a3b4c?amount=15.5")
    );
    assert!(!output.qr_ascii.is_empty());
    assert_eq!(output.result.status, AgenticToolCallStatus::Completed);
}

#[tokio::test]
async fn plugin_rejects_invalid_recipient_no_url_emitted() {
    let plugin = ZeroClawSolanaPlugin::new();
    let pay_req = SolanaPayRequest::new("INVALID_KEY").with_amount(25.0);
    let budget = AgentBudget {
        context_tokens: 1000,
        actions: 5,
        children: 2,
        wall_time_ms: 10000,
        cpu_ms: None,
    };
    let output = plugin.generate_solana_pay(pay_req, None, budget).await;
    assert_eq!(output.result.status, AgenticToolCallStatus::Rejected);
    assert!(output.url.is_empty());
    assert!(output.qr_ascii.is_empty());
}
