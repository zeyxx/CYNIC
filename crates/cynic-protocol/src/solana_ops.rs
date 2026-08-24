//! Solana operations and SOP sanitization types.
//!
//! Represents typed Solana Pay transfer requests, spend allowance grants,
//! and SOP input/sanitization structures for ZeroClaw and CYNIC integration.

use serde::{Deserialize, Serialize};

/// A Solana Pay transfer request URL builder conforming to the Solana Pay spec.
/// Format: `solana:<recipient>?amount=<amount>&spl-token=<spl-token>&reference=<ref>&label=<label>&message=<message>&memo=<memo>`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SolanaPayRequest {
    /// Recipient wallet public key (base58)
    pub recipient: String,
    /// Transfer amount in SOL or SPL token units (decimal formatted string or float)
    pub amount: Option<f64>,
    /// SPL token mint address (base58)
    pub spl_token: Option<String>,
    /// Reference public keys (base58) for tracking transaction on-chain via `getSignaturesForAddress`
    pub reference: Vec<String>,
    /// Human readable label for the transfer request
    pub label: Option<String>,
    /// Human readable message describing the purpose
    pub message: Option<String>,
    /// On-chain memo instruction payload
    pub memo: Option<String>,
}

impl SolanaPayRequest {
    pub fn new(recipient: impl Into<String>) -> Self {
        Self {
            recipient: recipient.into(),
            amount: None,
            spl_token: None,
            reference: Vec::new(),
            label: None,
            message: None,
            memo: None,
        }
    }

    pub fn with_amount(mut self, amount: f64) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn with_spl_token(mut self, token_mint: impl Into<String>) -> Self {
        self.spl_token = Some(token_mint.into());
        self
    }

    pub fn with_reference(mut self, reference_key: impl Into<String>) -> Self {
        self.reference.push(reference_key.into());
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    /// Renders the official Solana Pay transfer request URL string.
    pub fn to_url_string(&self) -> Result<String, String> {
        if self.recipient.trim().is_empty() {
            return Err("recipient public key cannot be empty".to_string());
        }

        let mut url_str = format!("solana:{}", self.recipient);
        let mut query_params: Vec<(String, String)> = Vec::new();

        if let Some(amt) = self.amount {
            if amt <= 0.0 {
                return Err("amount must be positive".to_string());
            }
            query_params.push(("amount".to_string(), format!("{amt}")));
        }

        if let Some(ref token) = self.spl_token {
            query_params.push(("spl-token".to_string(), token.clone()));
        }

        for ref_key in &self.reference {
            query_params.push(("reference".to_string(), ref_key.clone()));
        }

        if let Some(ref label) = self.label {
            query_params.push(("label".to_string(), label.clone()));
        }

        if let Some(ref msg) = self.message {
            query_params.push(("message".to_string(), msg.clone()));
        }

        if let Some(ref memo) = self.memo {
            query_params.push(("memo".to_string(), memo.clone()));
        }

        if !query_params.is_empty() {
            let mut serializer = url::form_urlencoded::Serializer::new(String::new());
            for (k, v) in query_params {
                serializer.append_pair(&k, &v);
            }
            url_str.push('?');
            url_str.push_str(&serializer.finish());
        }

        Ok(url_str)
    }
}

/// On-chain or local spend allowance grant for an agent delegate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllowanceGrant {
    pub grant_id: String,
    pub owner_pubkey: String,
    pub delegate_pubkey: String,
    pub spl_token: Option<String>,
    pub max_amount_per_period: f64,
    pub period_seconds: u64,
    pub remaining_allowance: f64,
    pub allowlist_recipients: Vec<String>,
    pub allowlist_mints: Vec<String>,
    pub expires_at: u64,
}

impl AllowanceGrant {
    /// Validates whether a proposed payment fits within the allowance spend limits and allowlists.
    pub fn validate_transfer(
        &self,
        recipient: &str,
        amount: f64,
        mint: Option<&str>,
    ) -> Result<(), String> {
        if amount <= 0.0 {
            return Err("transfer amount must be positive".to_string());
        }

        if amount > self.remaining_allowance {
            return Err(format!(
                "transfer amount {amount} exceeds remaining allowance {}",
                self.remaining_allowance
            ));
        }

        if !self.allowlist_recipients.is_empty()
            && !self.allowlist_recipients.iter().any(|r| r == recipient)
        {
            return Err(format!(
                "recipient {recipient} is not in recipient allowlist"
            ));
        }

        if let Some(m) = mint {
            if !self.allowlist_mints.is_empty() && !self.allowlist_mints.iter().any(|lm| lm == m) {
                return Err(format!("mint {m} is not in token mint allowlist"));
            }
        }

        Ok(())
    }
}

/// Input payload to be sanitized before executing an SOP step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SopSanitizationInput {
    pub content: String,
    pub channel: String,
    pub sender_id: String,
    pub requested_action: String,
    pub parameters: serde_json::Value,
}

/// Result of SOP input sanitization and threat analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SopSanitizationResult {
    pub is_safe: bool,
    pub risk_score: f64,
    pub detected_threats: Vec<String>,
    pub sanitized_content: String,
    pub requires_human_approval: bool,
    pub approval_reason: Option<String>,
}

pub const SOLANA_TAKEOVER_ATTESTATION_SCHEMA: &str = "cynic.solana_takeover_attestation.v1";

/// On-chain Solana commitment for a sovereign role takeover transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolanaTakeoverAttestation {
    pub schema: String,
    pub transition_id: String,
    pub cluster: String,
    pub from_node_id: Option<String>,
    pub to_node_id: String,
    pub role: String,
    pub fencing_token: u64,
    pub prior_fencing_token: u64,
    pub lease_signature_hex: String,
    pub commitment_memo: String,
}

impl SolanaTakeoverAttestation {
    pub fn from_transition_record(
        record: &crate::runtime_takeover::RoleTransitionRecord,
        cluster: &str,
    ) -> Self {
        let memo = format!(
            "CYNIC:TAKEOVER:{}:{}:{}:token={}:prior={}",
            cluster,
            record.role,
            record.to_node_id,
            record.fencing_token,
            record.prior_fencing_token
        );
        Self {
            schema: SOLANA_TAKEOVER_ATTESTATION_SCHEMA.to_string(),
            transition_id: record.transition_id.clone(),
            cluster: cluster.to_string(),
            from_node_id: record.from_node_id.clone(),
            to_node_id: record.to_node_id.clone(),
            role: record.role.clone(),
            fencing_token: record.fencing_token,
            prior_fencing_token: record.prior_fencing_token,
            lease_signature_hex: record.lease_signature_hex.clone(),
            commitment_memo: memo,
        }
    }

    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != SOLANA_TAKEOVER_ATTESTATION_SCHEMA {
            blockers.push("solana_takeover_attestation_schema_mismatch".to_string());
        }
        if self.to_node_id.trim().is_empty() {
            blockers.push("solana_takeover_target_node_empty".to_string());
        }
        if self.fencing_token <= self.prior_fencing_token {
            blockers.push("solana_takeover_fencing_token_not_monotonic".to_string());
        }
        if !self.commitment_memo.starts_with("CYNIC:TAKEOVER:") {
            blockers.push("solana_takeover_memo_malformed".to_string());
        }
        blockers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solana_pay_url_generation() {
        let req = SolanaPayRequest::new("7Xw7K1kPzSjW7Y2...")
            .with_amount(12.5)
            .with_spl_token("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
            .with_reference("RefKey123")
            .with_label("Store Purchase")
            .with_message("Order #412");

        let url = req.to_url_string().unwrap();
        assert!(url.starts_with("solana:7Xw7K1kPzSjW7Y2..."));
        assert!(url.contains("amount=12.5"));
        assert!(url.contains("spl-token=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"));
        assert!(url.contains("reference=RefKey123"));
        assert!(url.contains("label=Store+Purchase"));
    }

    #[test]
    fn allowance_grant_spend_limit_check() {
        let grant = AllowanceGrant {
            grant_id: "grant-1".into(),
            owner_pubkey: "owner1".into(),
            delegate_pubkey: "agent1".into(),
            spl_token: Some("USDC".into()),
            max_amount_per_period: 100.0,
            period_seconds: 86400,
            remaining_allowance: 50.0,
            allowlist_recipients: vec!["vendor1".into()],
            allowlist_mints: vec!["USDC".into()],
            expires_at: 1900000000,
        };

        assert!(
            grant
                .validate_transfer("vendor1", 20.0, Some("USDC"))
                .is_ok()
        );
        assert!(
            grant
                .validate_transfer("vendor1", 60.0, Some("USDC"))
                .is_err()
        );
        assert!(
            grant
                .validate_transfer("attacker", 10.0, Some("USDC"))
                .is_err()
        );
    }

    #[test]
    fn solana_takeover_attestation_derivation() {
        let record = crate::runtime_takeover::RoleTransitionRecord::new(
            "tx-001".to_string(),
            Some("node-a".to_string()),
            "node-b".to_string(),
            "cortex_primary".to_string(),
            42,
            41,
            10000,
            "a".repeat(128),
        );

        let attestation =
            SolanaTakeoverAttestation::from_transition_record(&record, "mainnet-beta");
        assert_eq!(attestation.fencing_token, 42);
        assert_eq!(attestation.to_node_id, "node-b");
        assert!(attestation.commitment_memo.contains("token=42:prior=41"));
        assert!(attestation.validate().is_empty());
    }
}
