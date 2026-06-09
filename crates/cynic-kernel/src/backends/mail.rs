//! Agentmail REST API backend for AI-first email accounts.
//! https://docs.agentmail.to
//!
//! Uses agentmail's HTTP API (not IMAP/SMTP) for managing agent email inboxes.
//! Auth: Bearer token via AGENTMAIL_API_KEY environment variable.

use crate::domain::mail::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

const AGENTMAIL_BASE_URL: &str = "https://api.agentmail.to/v0";
const DEFAULT_TIMEOUT_SECS: u64 = 10;

/// Agentmail REST API backend configuration.
#[derive(Debug, Clone)]
pub struct AgentmailConfig {
    pub api_key: String,
    pub inbox_id: String, // e.g., "cynic@agentmail.to"
    pub timeout: Duration,
}

impl AgentmailConfig {
    /// Load from environment variables.
    /// AGENTMAIL_API_KEY=sk_...
    /// AGENTMAIL_INBOX_ID=cynic@agentmail.to (or from MAIL_USERNAME)
    pub fn from_env(username_env: &str) -> Option<Self> {
        let api_key = std::env::var("AGENTMAIL_API_KEY").ok()?;
        let inbox_id = std::env::var(username_env).ok()?;
        let timeout = Duration::from_secs(DEFAULT_TIMEOUT_SECS);

        Some(AgentmailConfig {
            api_key,
            inbox_id,
            timeout,
        })
    }
}

/// Agentmail REST API client.
#[derive(Debug, Clone)]
pub struct AgentmailBackend {
    config: Arc<AgentmailConfig>,
    client: Client,
}

// ────────────────────────────────────────────────────────────
// Agentmail API Types
// ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct AgentmailMessage {
    #[serde(default)]
    message_id: String,
    #[serde(default)]
    inbox_id: String,
    #[serde(default)]
    thread_id: String,
    #[serde(default)]
    from: String,
    #[serde(default)]
    to: Vec<String>,
    #[serde(default)]
    subject: String,
    #[serde(default)]
    preview: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    html: Option<String>,
    #[serde(default)]
    timestamp: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    labels: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AgentmailListResponse {
    messages: Vec<AgentmailMessage>,
    #[serde(default)]
    count: usize,
}

#[derive(Debug, Serialize)]
struct SendMessageRequest {
    to: String,
    subject: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SendMessageResponse {
    id: String,
}

// ────────────────────────────────────────────────────────────

impl AgentmailBackend {
    pub fn new(config: AgentmailConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_default();

        AgentmailBackend {
            config: Arc::new(config),
            client,
        }
    }

    /// Build Authorization header.
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.config.api_key)
    }

    /// GET /inboxes/{inbox_id}/messages
    async fn list_messages_api(&self, limit: usize) -> Result<Vec<AgentmailMessage>, MailError> {
        let url = format!(
            "{}/inboxes/{}/messages?limit={}",
            AGENTMAIL_BASE_URL, self.config.inbox_id, limit
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| {
                warn!("Agentmail list failed: {e}");
                MailError::ConnectionFailed(format!("HTTP error: {e}"))
            })?;

        match response.status() {
            StatusCode::OK => {
                let body = response
                    .json::<AgentmailListResponse>()
                    .await
                    .map_err(|e| MailError::ParseError(format!("Invalid JSON response: {e}")))?;
                Ok(body.messages)
            }
            StatusCode::UNAUTHORIZED => Err(MailError::AuthenticationFailed),
            StatusCode::NOT_FOUND => Ok(vec![]),
            _ => Err(MailError::ConnectionFailed(format!(
                "HTTP {}",
                response.status()
            ))),
        }
    }

    /// POST /inboxes/{inbox_id}/messages/send
    async fn send_api(&self, to: &str, subject: &str, body: &str) -> Result<String, MailError> {
        let url = format!(
            "{}/inboxes/{}/messages/send",
            AGENTMAIL_BASE_URL, self.config.inbox_id
        );

        let req = SendMessageRequest {
            to: to.to_string(),
            subject: subject.to_string(),
            text: body.to_string(),
            html: None,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&req)
            .send()
            .await
            .map_err(|e| {
                warn!("Agentmail send failed: {e}");
                MailError::ConnectionFailed(format!("HTTP error: {e}"))
            })?;

        match response.status() {
            StatusCode::OK | StatusCode::CREATED => {
                let body = response
                    .json::<SendMessageResponse>()
                    .await
                    .map_err(|e| MailError::ParseError(format!("Invalid JSON response: {e}")))?;
                Ok(body.id)
            }
            StatusCode::UNAUTHORIZED => Err(MailError::AuthenticationFailed),
            _ => Err(MailError::ConnectionFailed(format!(
                "HTTP {}",
                response.status()
            ))),
        }
    }

    /// PATCH /inboxes/{inbox_id}/messages/{message_id} (add labels)
    async fn mark_read_api(&self, msg_id: &str) -> Result<(), MailError> {
        let url = format!(
            "{}/inboxes/{}/messages/{}",
            AGENTMAIL_BASE_URL, self.config.inbox_id, msg_id
        );

        let update = serde_json::json!({
            "add_labels": ["read"],
            "remove_labels": ["unread"]
        });

        let response = self
            .client
            .patch(&url)
            .header("Authorization", self.auth_header())
            .json(&update)
            .send()
            .await
            .map_err(|e| MailError::ConnectionFailed(format!("HTTP error: {e}")))?;

        match response.status() {
            StatusCode::OK => Ok(()),
            StatusCode::UNAUTHORIZED => Err(MailError::AuthenticationFailed),
            StatusCode::NOT_FOUND => Err(MailError::MessageNotFound),
            _ => Err(MailError::ConnectionFailed(format!(
                "HTTP {}",
                response.status()
            ))),
        }
    }
}

#[async_trait]
impl MailPort for AgentmailBackend {
    async fn health(&self) -> Result<MailHealth, MailError> {
        match self.list_messages_api(1).await {
            Ok(_) => Ok(MailHealth {
                connected: true,
                last_sync: Some(Utc::now()),
                quota_used_mb: 0,
                quota_total_mb: 0,
                unread_count: 0,
                error: None,
            }),
            Err(e) => {
                let error_msg = match e {
                    MailError::AuthenticationFailed => "Authentication failed".to_string(),
                    MailError::ConnectionFailed(msg) => msg,
                    _ => "Unknown error".to_string(),
                };
                Ok(MailHealth {
                    connected: false,
                    last_sync: None,
                    quota_used_mb: 0,
                    quota_total_mb: 0,
                    unread_count: 0,
                    error: Some(error_msg),
                })
            }
        }
    }

    async fn fetch_inbox(&self, filter: MailFilter) -> Result<Vec<MessageSummary>, MailError> {
        let limit = filter.limit.unwrap_or(20);
        let messages = self.list_messages_api(limit).await?;

        Ok(messages
            .into_iter()
            .map(|msg| {
                let ts = if msg.timestamp.is_empty() {
                    &msg.created_at
                } else {
                    &msg.timestamp
                };
                let date = DateTime::parse_from_rfc3339(ts)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                MessageSummary {
                    id: msg.message_id,
                    from: msg.from,
                    subject: msg.subject,
                    date,
                    folder: "INBOX".to_string(),
                    is_read: msg.labels.contains(&"read".to_string()),
                }
            })
            .collect())
    }

    async fn fetch_message(&self, _msg_id: String) -> Result<Message, MailError> {
        // TODO: Implement when agentmail provides full message endpoint
        Err(MailError::MessageNotFound)
    }

    async fn search(&self, _query: SearchQuery) -> Result<Vec<MessageSummary>, MailError> {
        // TODO: Implement when agentmail provides search endpoint
        Ok(vec![])
    }

    async fn mark_read(&self, msg_id: String) -> Result<(), MailError> {
        self.mark_read_api(&msg_id).await
    }

    async fn send_message(
        &self,
        to: String,
        subject: String,
        body: String,
    ) -> Result<String, MailError> {
        self.send_api(&to, &subject, &body).await
    }

    async fn sync(&self) -> Result<SyncStats, MailError> {
        let messages = self.list_messages_api(100).await?;
        let new_messages = messages.len();

        Ok(SyncStats {
            new_messages,
            deleted_messages: 0,
            updated_quota: MailHealth {
                connected: true,
                last_sync: Some(Utc::now()),
                quota_used_mb: 0,
                quota_total_mb: 0,
                unread_count: messages
                    .iter()
                    .filter(|m| !m.labels.contains(&"read".to_string()))
                    .count() as u32,
                error: None,
            },
        })
    }
}
