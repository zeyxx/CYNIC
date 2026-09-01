//! Notification Outbox Use Case (Slice C2).
//!
//! Defined in `infra/AGENT_FLEET_HOSTING_DESIGN.md` §Telegram : notification, jamais contrôle.
//! Outbox durable, redaction/classification, retry idempotent et receipt de livraison.

use cynic_protocol::{
    NotificationDeliveryStatus, NotificationOutboxEntry, validate_notification_outbox_entry,
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

pub const NOTIFICATION_OUTBOX_REPORT_SCHEMA: &str = "cynic.node.notification_outbox_report.v1";

/// Trait for durable outbox persistence.
pub trait NotificationOutboxStorePort: Send + Sync {
    fn enqueue(&self, entry: NotificationOutboxEntry) -> Result<(), String>;
    fn list_pending(&self) -> Result<Vec<NotificationOutboxEntry>, String>;
    fn mark_delivered(&self, notification_id: &str, receipt: &str) -> Result<(), String>;
    fn mark_failed(&self, notification_id: &str, reason: &str) -> Result<(), String>;
}

/// Trait for notification transport adapter (e.g. Telegram, Console).
pub trait NotificationTransportPort: Send + Sync {
    fn dispatch(&self, entry: &NotificationOutboxEntry) -> Result<String, String>;
}

/// In-memory outbox store for unit testing and local node spool.
#[derive(Default)]
pub struct InMemoryNotificationOutboxStore {
    entries: Mutex<Vec<NotificationOutboxEntry>>,
}

impl NotificationOutboxStorePort for InMemoryNotificationOutboxStore {
    fn enqueue(&self, entry: NotificationOutboxEntry) -> Result<(), String> {
        let violations = validate_notification_outbox_entry(&entry);
        if !violations.is_empty() {
            return Err(format!(
                "invalid_notification_entry: {}",
                violations.join(", ")
            ));
        }
        let mut list = self.entries.lock().map_err(|e| e.to_string())?;
        // Deduplicate by idempotency key
        if list
            .iter()
            .any(|e| e.idempotency_key == entry.idempotency_key)
        {
            return Ok(());
        }
        list.push(entry);
        Ok(())
    }

    fn list_pending(&self) -> Result<Vec<NotificationOutboxEntry>, String> {
        let list = self.entries.lock().map_err(|e| e.to_string())?;
        Ok(list
            .iter()
            .filter(|e| matches!(e.delivery_status, NotificationDeliveryStatus::Pending))
            .cloned()
            .collect())
    }

    fn mark_delivered(&self, notification_id: &str, receipt: &str) -> Result<(), String> {
        let mut list = self.entries.lock().map_err(|e| e.to_string())?;
        if let Some(entry) = list
            .iter_mut()
            .find(|e| e.notification_id == notification_id)
        {
            entry.delivery_status = NotificationDeliveryStatus::Delivered;
            entry.delivery_receipt = Some(receipt.to_string());
            Ok(())
        } else {
            Err(format!("notification_not_found:{notification_id}"))
        }
    }

    fn mark_failed(&self, notification_id: &str, reason: &str) -> Result<(), String> {
        let mut list = self.entries.lock().map_err(|e| e.to_string())?;
        if let Some(entry) = list
            .iter_mut()
            .find(|e| e.notification_id == notification_id)
        {
            entry.delivery_status = NotificationDeliveryStatus::Failed;
            entry.delivery_receipt = Some(format!("failed:{reason}"));
            Ok(())
        } else {
            Err(format!("notification_not_found:{notification_id}"))
        }
    }
}

/// Result of processing the notification outbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationOutboxProcessingReport {
    pub schema: String,
    pub processed_count: usize,
    pub delivered_count: usize,
    pub failed_count: usize,
    pub receipts: Vec<String>,
}

/// Outbox processor reconciling pending entries through an injected transport.
pub fn process_notification_outbox(
    store: &dyn NotificationOutboxStorePort,
    transport: &dyn NotificationTransportPort,
) -> Result<NotificationOutboxProcessingReport, String> {
    let pending = store.list_pending()?;
    let mut delivered_count = 0;
    let mut failed_count = 0;
    let mut receipts = Vec::new();

    for entry in pending {
        match transport.dispatch(&entry) {
            Ok(receipt) => {
                store.mark_delivered(&entry.notification_id, &receipt)?;
                receipts.push(receipt);
                delivered_count += 1;
            }
            Err(error) => {
                store.mark_failed(&entry.notification_id, &error)?;
                failed_count += 1;
            }
        }
    }

    Ok(NotificationOutboxProcessingReport {
        schema: NOTIFICATION_OUTBOX_REPORT_SCHEMA.to_string(),
        processed_count: delivered_count + failed_count,
        delivered_count,
        failed_count,
        receipts,
    })
}
/// Helper to escape raw HTML text for Telegram HTML parse mode.
fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Canonical Telegram notification transport adapter posting to Telegram Bot API.
///
/// Implements `NotificationTransportPort`.
pub struct HttpTelegramNotificationTransport {
    bot_token: String,
    chat_id: String,
    client: reqwest::blocking::Client,
}

impl std::fmt::Debug for HttpTelegramNotificationTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpTelegramNotificationTransport")
            .field("bot_token", &"***REDACTED***")
            .field("chat_id", &self.chat_id)
            .finish_non_exhaustive()
    }
}

impl HttpTelegramNotificationTransport {
    pub fn new(
        bot_token: impl Into<String>,
        chat_id: impl Into<String>,
        timeout: std::time::Duration,
    ) -> Result<Self, String> {
        let bot_token = bot_token.into();
        let chat_id = chat_id.into();
        if bot_token.trim().is_empty() {
            return Err("telegram_bot_token_empty".to_string());
        }
        if chat_id.trim().is_empty() {
            return Err("telegram_chat_id_empty".to_string());
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("http_client_init_failed:{e}"))?;

        Ok(Self {
            bot_token,
            chat_id,
            client,
        })
    }

    pub fn from_env() -> Result<Self, String> {
        let bot_token = std::env::var("CYNIC_TELEGRAM_BOT_TOKEN")
            .map_err(|_| "CYNIC_TELEGRAM_BOT_TOKEN_not_set".to_string())?;
        let chat_id = std::env::var("CYNIC_TELEGRAM_CHAT_ID")
            .map_err(|_| "CYNIC_TELEGRAM_CHAT_ID_not_set".to_string())?;
        Self::new(bot_token, chat_id, std::time::Duration::from_secs(5))
    }
}

impl NotificationTransportPort for HttpTelegramNotificationTransport {
    fn dispatch(&self, entry: &NotificationOutboxEntry) -> Result<String, String> {
        let text = format!(
            "<b>[CYNIC Watchdog Alert]</b>\n\
             <b>Summary:</b> {}\n\
             <b>ID:</b> <code>{}</code>\n\
             <b>Event:</b> {:?}\n\
             <b>Idempotency:</b> <code>{}</code>",
            html_escape(&entry.summary),
            entry.notification_id,
            entry.referenced_event.kind,
            entry.idempotency_key,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        let payload = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML"
        });

        let res = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .map_err(|e| format!("telegram_api_request_failed:{e}"))?;

        if !res.status().is_success() {
            return Err(format!("telegram_api_http_error:{}", res.status()));
        }

        let body: serde_json::Value = res
            .json()
            .map_err(|e| format!("telegram_api_invalid_json:{e}"))?;

        if body.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
            let desc = body
                .get("description")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown_error");
            return Err(format!("telegram_api_rejected:{desc}"));
        }

        let message_id = body
            .get("result")
            .and_then(|r| r.get("message_id"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);

        Ok(format!(
            "telegram:msg_id:{message_id}:{}",
            entry.notification_id
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        NotificationChannel, NotificationContentClassification, NotificationReferencedEvent,
        NotificationReferencedEventKind,
    };

    struct MockTelegramTransport {
        fail: bool,
    }

    impl NotificationTransportPort for MockTelegramTransport {
        fn dispatch(&self, entry: &NotificationOutboxEntry) -> Result<String, String> {
            if self.fail {
                Err("network_timeout".to_string())
            } else {
                Ok(format!("telegram:msg_id:1001:{}", entry.notification_id))
            }
        }
    }

    fn sample_entry(id: &str, idem: &str) -> NotificationOutboxEntry {
        NotificationOutboxEntry {
            schema: cynic_protocol::NOTIFICATION_OUTBOX_ENTRY_SCHEMA.to_string(),
            notification_id: id.to_string(),
            referenced_event: NotificationReferencedEvent {
                kind: NotificationReferencedEventKind::WorkloadCompleted,
                event_id: "evt-1".to_string(),
                workload_id: "wl-1".to_string(),
                commit_sha: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
                build_verdict: Some("passed".to_string()),
                proof_ref: Some("proof://build/1".to_string()),
            },
            channel: NotificationChannel::Telegram,
            classification: NotificationContentClassification::PublicRedacted,
            idempotency_key: idem.to_string(),
            summary: "Build passed for PR commit 0123456789abcdef0123456789abcdef01234567"
                .to_string(),
            delivery_status: NotificationDeliveryStatus::Pending,
            delivery_receipt: None,
            created_at_unix_ms: 1788267855000,
        }
    }

    #[test]
    fn process_outbox_delivers_successfully_and_records_receipt() {
        let store = InMemoryNotificationOutboxStore::default();
        store.enqueue(sample_entry("n1", "idem-1")).unwrap();
        store.enqueue(sample_entry("n2", "idem-2")).unwrap();

        let transport = MockTelegramTransport { fail: false };
        let report = process_notification_outbox(&store, &transport).unwrap();

        assert_eq!(report.delivered_count, 2);
        assert_eq!(report.failed_count, 0);
        assert_eq!(report.receipts.len(), 2);
        assert!(report.receipts[0].starts_with("telegram:msg_id:1001:n1"));

        // Verify outbox is now empty
        let remaining = store.list_pending().unwrap();
        assert!(remaining.is_empty());
    }

    #[test]
    fn process_outbox_handles_transport_failure() {
        let store = InMemoryNotificationOutboxStore::default();
        store.enqueue(sample_entry("n1", "idem-1")).unwrap();

        let transport = MockTelegramTransport { fail: true };
        let report = process_notification_outbox(&store, &transport).unwrap();

        assert_eq!(report.delivered_count, 0);
        assert_eq!(report.failed_count, 1);
    }
    #[test]
    fn html_escape_escapes_special_characters() {
        assert_eq!(
            html_escape("<alert> & 'warning'"),
            "&lt;alert&gt; &amp; 'warning'"
        );
    }

    #[test]
    fn telegram_transport_rejects_empty_token_or_chat_id() {
        let res =
            HttpTelegramNotificationTransport::new("", "12345", std::time::Duration::from_secs(1));
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "telegram_bot_token_empty");

        let res2 =
            HttpTelegramNotificationTransport::new("token", "", std::time::Duration::from_secs(1));
        assert!(res2.is_err());
        assert_eq!(res2.unwrap_err(), "telegram_chat_id_empty");
    }
}
