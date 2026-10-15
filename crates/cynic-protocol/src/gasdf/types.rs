//! GASdf subscription types — pure vocabulary (zero I/O).
//!
//! Cross-family contract for the GASdf payment abstraction vertical.
//! Lives in cynic-protocol because it is contract vocabulary, not Solana-specific.
//! Adapters (HTTP, DB, DEX, signer) consume these types; they live elsewhere.

use serde::{Deserialize, Serialize};

pub const GASDF_PAYMENT_TOKENS_SCHEMA: &str = "cynic.gasdf.payment_tokens.v1";
pub const GASDF_PLANS_SCHEMA: &str = "cynic.gasdf.plans.v1";
pub const GASDF_SESSION_SCHEMA: &str = "cynic.gasdf.subscription_session.v1";
pub const GASDF_QUOTE_SCHEMA: &str = "cynic.gasdf.payment_quote.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentTokenKind {
    Native,
    Stable,
    Memecoin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentToken {
    pub mint: String,
    pub symbol: String,
    pub decimals: u8,
    pub kind: PaymentTokenKind,
    pub enabled: bool,
    pub min_amount: u64,
    pub slippage_bps_default: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Open,
    Quoted,
    Pending,
    Settled,
    Expired,
    Rejected,
    Refunded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    pub app_id: String,
    pub plan_id: String,
    pub name: String,
    pub price_asdf_base_units: u64,
    pub duration_seconds: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionSession {
    pub schema: String,
    pub session_id: String,
    pub app_id: String,
    pub plan_id: String,
    pub buyer_pubkey: String,
    pub input_token_mint: Option<String>,
    pub status: SessionStatus,
    pub created_at: u64,
    pub expires_at: u64,
    pub settled_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentQuote {
    pub schema: String,
    pub quote_id: String,
    pub session_id: String,
    pub input_token_mint: String,
    pub input_amount: u64,
    pub estimated_asdf: u64,
    pub slippage_bps: u16,
    pub fees_base_units: u64,
    pub gas_sponsored: bool,
    pub expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentTokenLimits {
    pub max_slippage_bps: u16,
    pub quote_ttl_seconds: u64,
    pub min_quote_amount_asdf: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentTokenRegistry {
    pub schema: String,
    pub tokens: Vec<PaymentToken>,
    pub limits: PaymentTokenLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanRegistry {
    pub schema: String,
    pub plans: Vec<SubscriptionPlan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenDecision {
    Allowed,
    Disabled,
    Unknown,
    BelowMinimum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionQuoteDecision {
    Accept {
        input_amount: u64,
        estimated_asdf: u64,
        slippage_bps: u16,
        gas_sponsored: bool,
    },
    Reject {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionTransition {
    ToQuoted,
    ToPending,
    ToSettled {
        settled_at: u64,
    },
    ToExpired,
    ToRejected {
        reason: String,
    },
    ToRefunded {
        refunded_at: u64,
    },
    NoOp {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementDecision {
    Settle {
        settled_at: u64,
    },
    PendingConfirmation,
    Shortfall {
        received_asdf: u64,
        required_asdf: u64,
    },
    Rejected {
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    QuoteRequested,
    PaymentSubmitted,
    PaymentConfirmed {
        at: u64,
    },
    TtlExpired,
    RefundRequested {
        at: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn payment_token_registry_parses() {
        let raw = fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../infra/projects/gasdf/payment-tokens.json"
        ))
        .expect("registry json readable");
        let reg: PaymentTokenRegistry = serde_json::from_str(&raw).expect("valid registry");
        assert_eq!(reg.schema, "cynic.gasdf.payment_tokens.v1");
        assert!(!reg.tokens.is_empty());
        assert!(reg.tokens.iter().any(|t| t.mint == "SOL"));
    }

    #[test]
    fn plan_registry_parses() {
        let raw = fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../infra/projects/gasdf/plans.json"
        ))
        .expect("plans json readable");
        let reg: PlanRegistry = serde_json::from_str(&raw).expect("valid plans");
        assert_eq!(reg.schema, "cynic.gasdf.plans.v1");
        assert!(!reg.plans.is_empty());
    }
}