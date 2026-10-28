//! Notification outbox store port (pure trait).

use cynic_protocol::NotificationOutboxEntry;

pub trait NotificationOutboxStorePort: Send + Sync {
    fn enqueue(&self, entry: NotificationOutboxEntry) -> Result<(), String>;
    fn list_pending(&self) -> Result<Vec<NotificationOutboxEntry>, String>;
    fn mark_delivered(&self, notification_id: &str, receipt: &str) -> Result<(), String>;
    fn mark_failed(&self, notification_id: &str, reason: &str) -> Result<(), String>;
}