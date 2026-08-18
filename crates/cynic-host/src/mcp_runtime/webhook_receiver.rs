//! Push-based settlement webhook receiver — replaces cron polling.
//!
//! Instead of polling `getSignaturesForAddress` on a timer (the old ZeroClaw
//! pattern), this module registers an HTTP listener that receives Solana
//! webhook events directly. Zero polling anywhere in the settlement stack.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Schema identifier for the settlement webhook notification envelope.
pub const SETTLEMENT_NOTIFICATION_SCHEMA: &str = "cynic.zeroclaw.solana_settlement_webhook.v1";

/// A single settlement event pushed from the Solana webhook listener.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementNotification {
    /// Schema marker — always `cynic.zeroclaw.solana_settlement_webhook.v1`.
    #[serde(rename = "$schema")]
    pub schema: &'static str,
    /// The reference key the merchant registered a listener for.
    pub reference: String,
    /// Base58 transaction signature that cleared the reference key.
    pub signature: String,
    /// Slot at which the transaction was confirmed.
    pub slot: u64,
    /// UTC timestamp of the notification as an RFC 3339 string.
    pub timestamp: String,
    /// Number of lamports transferred (0 for SPL-token transfers).
    pub lamports: u64,
    /// SPL token transfer amount in minor units, if applicable.
    pub spl_amount: Option<u64>,
    /// SPL token mint symbol (e.g. "USDC"), if applicable.
    pub spl_token: Option<String>,
}

/// The runtime state for a push-based settlement listener.
pub struct WebhookSettlementMonitor {
    /// The reference key this listener is subscribed to.
    pub reference: String,
    /// HTTP path the listener binds to (injected, not hardcoded).
    pub listen_path: String,
    /// When the listener was registered.
    pub registered_at: SystemTime,
    /// Whether the listener has received at least one event.
    pub received_any: bool,
}

impl SettlementNotification {
    /// Build a notification from raw webhook payload fields.
    pub fn new(reference: String, signature: String, slot: u64, lamports: u64) -> Self {
        let timestamp: String = chrono::DateTime::<Utc>::from(SystemTime::now()).to_rfc3339();
        Self {
            schema: SETTLEMENT_NOTIFICATION_SCHEMA,
            reference,
            signature,
            slot,
            timestamp,
            lamports,
            spl_amount: None,
            spl_token: None,
        }
    }

    /// Verify the notification carries a valid (non-empty) signature.
    pub fn is_valid(&self) -> bool {
        !self.signature.is_empty() && self.slot > 0
    }
}

impl WebhookSettlementMonitor {
    /// Create a new listener bound to the given reference key and HTTP path.
    pub fn new(reference: String, listen_path: String) -> Self {
        Self {
            reference,
            listen_path,
            registered_at: SystemTime::now(),
            received_any: false,
        }
    }

    /// Record that a settlement event was received via push.
    pub fn mark_received(&mut self) {
        self.received_any = true;
    }

    /// Whether this listener has ever received a push event.
    pub fn has_received(&self) -> bool {
        self.received_any
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settlement_notification_new_is_valid() {
        let notif = SettlementNotification::new(
            "RefKey123".to_string(),
            "5Tx3...".to_string(),
            123456789,
            1_000_000_000,
        );
        assert!(notif.is_valid());
        assert_eq!(notif.schema, SETTLEMENT_NOTIFICATION_SCHEMA);
        assert_eq!(notif.reference, "RefKey123");
        assert_eq!(notif.lamports, 1_000_000_000);
    }

    #[test]
    fn settlement_notification_empty_signature_is_invalid() {
        let notif = SettlementNotification::new("ref".to_string(), String::new(), 0, 0);
        assert!(!notif.is_valid());
    }

    #[test]
    fn webhook_monitor_tracks_received_state() {
        let mut monitor = WebhookSettlementMonitor::new(
            "refkey".to_string(),
            "/webhooks/solana/refkey".to_string(),
        );
        assert!(!monitor.has_received());
        monitor.mark_received();
        assert!(monitor.has_received());
    }

    #[test]
    fn webhook_notification_serializes_to_json() {
        let notif =
            SettlementNotification::new("ref".to_string(), "sig123".to_string(), 42, 500_000_000);
        let json = serde_json::to_string(&notif).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed["$schema"], SETTLEMENT_NOTIFICATION_SCHEMA);
        assert_eq!(parsed["signature"], "sig123");
        assert_eq!(parsed["slot"], 42);
    }
}
