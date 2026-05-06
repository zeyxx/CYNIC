//! MailPort — domain contract for organism email identity.
//! Hexagonal architecture: domain abstraction for mail operations across providers
//! (agentmail.to REST API, Gmail IMAP/SMTP, Outlook, etc.)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Message summary (lightweight) — for inbox listing and search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSummary {
    pub id: String,
    pub from: String,
    pub subject: String,
    pub date: DateTime<Utc>,
    pub folder: String,
    pub is_read: bool,
}

/// Full message with headers and body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub from: String,
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    pub html_body: Option<String>,
    pub date: DateTime<Utc>,
    pub folder: String,
    pub is_read: bool,
    /// Headers as BTreeMap for deterministic serialization (K19).
    pub headers: BTreeMap<String, String>,
}

/// Search filter for inbox queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MailFilter {
    pub folder: Option<String>,
    pub is_unread: Option<bool>,
    pub limit: Option<usize>,
}

/// Health status of mail connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailHealth {
    pub connected: bool,
    pub last_sync: Option<DateTime<Utc>>,
    pub quota_used_mb: u32,
    pub quota_total_mb: u32,
    pub unread_count: u32,
    pub error: Option<String>,
}

/// Sync operation statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStats {
    pub new_messages: usize,
    pub deleted_messages: usize,
    pub updated_quota: MailHealth,
}

/// Search query for filtering messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub subject: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub folder: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub is_unread: Option<bool>,
    pub limit: Option<usize>,
}

/// Error type for mail operations.
#[derive(Debug, Clone)]
pub enum MailError {
    ConnectionFailed(String),
    AuthenticationFailed,
    MessageNotFound,
    InvalidQuery,
    IOError(String),
    ParseError(String),
    QuotaExceeded,
}

impl std::fmt::Display for MailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailError::ConnectionFailed(msg) => write!(f, "Mail connection failed: {msg}"),
            MailError::AuthenticationFailed => write!(f, "Mail authentication failed"),
            MailError::MessageNotFound => write!(f, "Message not found"),
            MailError::InvalidQuery => write!(f, "Invalid search query"),
            MailError::IOError(msg) => write!(f, "Mail I/O error: {msg}"),
            MailError::ParseError(msg) => write!(f, "Mail parse error: {msg}"),
            MailError::QuotaExceeded => write!(f, "Quota exceeded"),
        }
    }
}

impl std::error::Error for MailError {}

/// Domain port for email operations (send, receive, check inbox).
/// Implemented by providers: agentmail.to, Gmail, Outlook, etc.
#[async_trait]
pub trait MailPort: Send + Sync + std::fmt::Debug {
    /// Check connection status and mailbox health.
    async fn health(&self) -> Result<MailHealth, MailError>;

    /// Fetch inbox messages with optional filter.
    async fn fetch_inbox(&self, filter: MailFilter) -> Result<Vec<MessageSummary>, MailError>;

    /// Fetch full message with headers and body.
    async fn fetch_message(&self, msg_id: String) -> Result<Message, MailError>;

    /// Search messages by subject, from, date, etc.
    async fn search(&self, query: SearchQuery) -> Result<Vec<MessageSummary>, MailError>;

    /// Mark message as read.
    async fn mark_read(&self, msg_id: String) -> Result<(), MailError>;

    /// Send an email.
    async fn send_message(
        &self,
        to: String,
        subject: String,
        body: String,
    ) -> Result<String, MailError>;

    /// Synchronize inbox: fetch new/deleted messages, update quota.
    async fn sync(&self) -> Result<SyncStats, MailError>;
}
