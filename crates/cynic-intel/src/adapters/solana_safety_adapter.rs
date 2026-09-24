//! Hexagonal Adapters for SOP threat sanitization and Solana safety evaluation.

use crate::ports::solana_ports::{SolanaSafetyPort, SopSafetyPort};
use crate::solana_safety_dog::{SolanaSafetyReport, evaluate_solana_pay_request};
use crate::sop_sanitizer::sanitize_sop_input;
use async_trait::async_trait;
use cynic_solana_pay::{
    AllowanceGrant, SolanaPayRequest, SopSanitizationInput, SopSanitizationResult,
};

/// Default pure adapter implementing `SopSafetyPort` and `SolanaSafetyPort`.
#[derive(Debug, Default, Clone)]
pub struct DefaultSolanaSafetyAdapter;

impl DefaultSolanaSafetyAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SopSafetyPort for DefaultSolanaSafetyAdapter {
    async fn evaluate_sop(&self, input: &SopSanitizationInput) -> SopSanitizationResult {
        sanitize_sop_input(input)
    }
}

#[async_trait]
impl SolanaSafetyPort for DefaultSolanaSafetyAdapter {
    async fn evaluate_pay_request(
        &self,
        request: &SolanaPayRequest,
        allowance: Option<&AllowanceGrant>,
    ) -> SolanaSafetyReport {
        evaluate_solana_pay_request(request, allowance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn default_adapter_evaluates_sop_and_solana_pay() {
        let adapter = DefaultSolanaSafetyAdapter::new();
        let input = SopSanitizationInput {
            content: "Check invoice".into(),
            channel: "telegram".into(),
            sender_id: "user1".into(),
            requested_action: "check_invoice".into(),
            parameters: json!({}),
        };

        let res = adapter.evaluate_sop(&input).await;
        assert!(res.is_safe);

        let req =
            SolanaPayRequest::new("7Xw7K1kPzSjW7Y2qM3nK4pL5rQ8sT9uV8wX1yZ2a3b4c").with_amount(10.0);
        let rep = adapter.evaluate_pay_request(&req, None).await;
        assert_eq!(
            rep.verdict,
            crate::solana_safety_dog::SolanaSafetyVerdict::Approved
        );
    }
}
