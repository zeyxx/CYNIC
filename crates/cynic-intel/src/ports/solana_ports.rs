//! Hexagonal Ports for Solana Operations, RPC observation, and SOP safety evaluation.

use crate::domain_enrichment::{EnrichmentError, TokenData};
use crate::solana_safety_dog::SolanaSafetyReport;
use async_trait::async_trait;
use cynic_solana_pay::{
    AllowanceGrant, SolanaPayRequest, SopSanitizationInput, SopSanitizationResult,
};
use serde::{Deserialize, Serialize};

/// Port for evaluating SOP safety and threat sanitization.
#[async_trait]
pub trait SopSafetyPort: Send + Sync {
    async fn evaluate_sop(&self, input: &SopSanitizationInput) -> SopSanitizationResult;
}

/// Port for evaluating Solana Pay transfer requests and spend allowances.
#[async_trait]
pub trait SolanaSafetyPort: Send + Sync {
    async fn evaluate_pay_request(
        &self,
        request: &SolanaPayRequest,
        allowance: Option<&AllowanceGrant>,
    ) -> SolanaSafetyReport;
}

/// Port for observing Solana RPC blockchain state (signatures, balances, transactions).
/// NOTE: Prefer `SolanaSettlementPort` (push/webhook) over polling via this port.
#[async_trait]
pub trait SolanaRpcPort: Send + Sync {
    async fn get_signatures_for_address(&self, address: &str) -> Result<Vec<String>, String>;
    async fn get_balance(&self, address: &str) -> Result<u64, String>;
}

/// Port for on-chain token data enrichment (replaces direct Helius coupling).
#[async_trait]
pub trait TokenEnricherPort: Send + Sync {
    async fn enrich(&self, mint_address: &str) -> Result<Option<TokenData>, EnrichmentError>;
}

/// Webhook-driven settlement notification payload (push — zero polling).
/// Fired by a Solana transaction indexer / RPC node webhook when a
/// settlement signature appears on a watched reference key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettlementNotification {
    pub reference_key: String,
    pub signature: String,
    pub slot: u64,
    pub amount: Option<f64>,
    pub token_mint: Option<String>,
    pub timestamp_unix_ms: u64,
}

/// Port for receiving push-based settlement notifications (eliminates cron polling).
/// Implementors register interest in a reference key, then receive notifications
/// via the callback. This replaces the previous 60s `getSignaturesForAddress` poll.
#[async_trait]
pub trait SolanaSettlementPort: Send + Sync {
    /// Register a reference key for push notifications. Returns immediately
    /// (non-blocking). The runtime indexer pushes notifications to the callback.
    fn register_settlement_watch(&mut self, reference_key: &str) -> Result<(), String>;

    /// Process one settlement notification through the safety firewall.
    async fn process_settlement(&self, note: SettlementNotification) -> SolanaSafetyReport;
}

/// Mock RPC Adapter implementing `SolanaRpcPort` for deterministic offline testing.
#[derive(Debug, Default, Clone)]
pub struct MockSolanaRpcAdapter {
    pub mock_signatures: Vec<String>,
    pub mock_balance: u64,
    pub should_fail: bool,
}

impl MockSolanaRpcAdapter {
    pub fn new(mock_signatures: Vec<String>, mock_balance: u64) -> Self {
        Self {
            mock_signatures,
            mock_balance,
            should_fail: false,
        }
    }
}

#[async_trait]
impl SolanaRpcPort for MockSolanaRpcAdapter {
    async fn get_signatures_for_address(&self, _address: &str) -> Result<Vec<String>, String> {
        if self.should_fail {
            return Err("mock RPC failure".to_string());
        }
        Ok(self.mock_signatures.clone())
    }

    async fn get_balance(&self, _address: &str) -> Result<u64, String> {
        if self.should_fail {
            return Err("mock RPC failure".to_string());
        }
        Ok(self.mock_balance)
    }
}

/// Webhook-driven settlement adapter — implements `SolanaSettlementPort`
/// without any polling. Settlements are pushed by an external indexer.
#[derive(Debug, Default, Clone)]
pub struct WebhookSolanaSettlementAdapter {
    watched_keys: Vec<String>,
}

impl WebhookSolanaSettlementAdapter {
    pub fn new() -> Self {
        Self {
            watched_keys: Vec::new(),
        }
    }
}

#[async_trait]
impl SolanaSettlementPort for WebhookSolanaSettlementAdapter {
    fn register_settlement_watch(&mut self, reference_key: &str) -> Result<(), String> {
        if self.watched_keys.iter().any(|k| k == reference_key) {
            return Err(format!("already watching key: {reference_key}"));
        }
        self.watched_keys.push(reference_key.to_string());
        Ok(())
    }

    async fn process_settlement(&self, note: SettlementNotification) -> SolanaSafetyReport {
        // Build a synthetic SolanaPayRequest for safety evaluation from the notification
        let req = SolanaPayRequest::new("7Xw7K1kPzSjW7Y2mM3nK4pL5rQ8sT9uV8wW1yZ2a3b4c")
            .with_amount(note.amount.unwrap_or(0.0))
            .with_spl_token(note.token_mint.unwrap_or_default())
            .with_reference(note.reference_key.clone());
        crate::solana_safety_dog::evaluate_solana_pay_request(&req, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_solana_rpc_adapter_works() {
        let adapter = MockSolanaRpcAdapter::new(vec!["sig1".into(), "sig2".into()], 1_000_000);
        let sigs = adapter
            .get_signatures_for_address("address1")
            .await
            .unwrap();
        assert_eq!(sigs.len(), 2);
        assert_eq!(sigs[0], "sig1");

        let bal = adapter.get_balance("address1").await.unwrap();
        assert_eq!(bal, 1_000_000);
    }

    #[tokio::test]
    async fn webhook_settlement_adapter_registers_and_processes() {
        let mut adapter = WebhookSolanaSettlementAdapter::new();
        assert!(adapter.register_settlement_watch("RefKeyDemo2048").is_ok());
        assert!(adapter.register_settlement_watch("RefKeyDemo2048").is_err()); // duplicate

        let note = SettlementNotification {
            reference_key: "RefKeyDemo2048".into(),
            signature: "5SigExample".into(),
            slot: 320_000_000,
            amount: Some(5.0),
            token_mint: Some("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into()),
            timestamp_unix_ms: 1722000000000,
        };

        let report = adapter.process_settlement(note).await;
        assert_eq!(
            report.verdict,
            crate::solana_safety_dog::SolanaSafetyVerdict::Approved
        );
    }
}
