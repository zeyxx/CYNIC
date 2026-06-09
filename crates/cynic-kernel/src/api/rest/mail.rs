//! Mail REST API — routes for inbox operations.
//! Handlers delegate to the mail backend (agentmail.to, Gmail IMAP, etc.).

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::rest::AppState;
use crate::domain::mail::{MailFilter, MailHealth, MessageSummary, SearchQuery};

// ── RESPONSE TYPES ──────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub connected: bool,
    pub unread_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InboxResponse {
    pub messages: Vec<MessageSummary>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct SendResponse {
    pub message_id: String,
    pub sent: String,
}

#[derive(Debug, Serialize)]
pub struct UnreadResponse {
    pub unread: u32,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub new_messages: usize,
    pub deleted_messages: usize,
    #[serde(flatten)]
    pub quota: MailHealth,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<MessageSummary>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct ErrorMessageResponse {
    pub error: String,
}

// ── REQUEST TYPES ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InboxQuery {
    pub folder: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SendRequest {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub subject: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub folder: Option<String>,
    pub is_unread: Option<bool>,
    pub limit: Option<usize>,
}

// ── HANDLERS ────────────────────────────────────────────────

/// GET /mail/health — Check mail service connection status (no auth required for minimal info)
pub async fn health(State(app_state): State<Arc<AppState>>) -> (StatusCode, Json<HealthResponse>) {
    match &app_state.mail {
        Some(mail) => match mail.health().await {
            Ok(health) => (
                StatusCode::OK,
                Json(HealthResponse {
                    connected: health.connected,
                    unread_count: health.unread_count,
                    error: health.error,
                }),
            ),
            Err(e) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthResponse {
                    connected: false,
                    unread_count: 0,
                    error: Some(format!("{e:?}")),
                }),
            ),
        },
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                connected: false,
                unread_count: 0,
                error: Some("Mail service not configured".to_string()),
            }),
        ),
    }
}

/// GET /mail/inbox?folder=INBOX&limit=20 — Fetch inbox messages
pub async fn inbox(
    State(app_state): State<Arc<AppState>>,
    Query(params): Query<InboxQuery>,
) -> Result<Json<InboxResponse>, StatusCode> {
    let mail = app_state
        .mail
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let filter = MailFilter {
        folder: params.folder,
        is_unread: None,
        limit: params.limit,
    };

    let messages = mail
        .fetch_inbox(filter)
        .await
        .map_err(|_err| StatusCode::SERVICE_UNAVAILABLE)?;

    let count = messages.len();
    Ok(Json(InboxResponse { messages, count }))
}

/// GET /mail/messages/{id} — Fetch full message with headers
pub async fn fetch_message(
    State(app_state): State<Arc<AppState>>,
    Path(msg_id): Path<String>,
) -> Result<Json<crate::domain::mail::Message>, StatusCode> {
    let mail = app_state
        .mail
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    mail.fetch_message(msg_id)
        .await
        .map(Json)
        .map_err(|_err| StatusCode::NOT_FOUND)
}

/// POST /mail/send — Send an email
pub async fn send(
    State(app_state): State<Arc<AppState>>,
    Json(req): Json<SendRequest>,
) -> Result<Json<SendResponse>, StatusCode> {
    let mail = app_state
        .mail
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let message_id = mail
        .send_message(req.to, req.subject, req.body)
        .await
        .map_err(|_err| StatusCode::SERVICE_UNAVAILABLE)?;

    Ok(Json(SendResponse {
        message_id,
        sent: chrono::Utc::now().to_rfc3339(),
    }))
}

/// POST /mail/sync — Synchronize inbox (fetch new/deleted messages, update quota)
pub async fn sync(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<SyncResponse>, StatusCode> {
    let mail = app_state
        .mail
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let stats = mail
        .sync()
        .await
        .map_err(|_err| StatusCode::SERVICE_UNAVAILABLE)?;

    Ok(Json(SyncResponse {
        new_messages: stats.new_messages,
        deleted_messages: stats.deleted_messages,
        quota: stats.updated_quota,
    }))
}

/// POST /mail/mark-read/{id} — Mark message as read
pub async fn mark_read(
    State(app_state): State<Arc<AppState>>,
    Path(msg_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let mail = app_state
        .mail
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    mail.mark_read(msg_id)
        .await
        .map_err(|_err| StatusCode::SERVICE_UNAVAILABLE)?;

    Ok(StatusCode::OK)
}

/// GET /mail/unread — Count unread messages
pub async fn unread(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<UnreadResponse>, StatusCode> {
    let mail = app_state
        .mail
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let health = mail
        .health()
        .await
        .map_err(|_err| StatusCode::SERVICE_UNAVAILABLE)?;

    Ok(Json(UnreadResponse {
        unread: health.unread_count,
    }))
}

/// POST /mail/search — Search messages with filters
pub async fn search(
    State(app_state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let mail = app_state
        .mail
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let query = SearchQuery {
        subject: req.subject,
        from: req.from,
        to: req.to,
        folder: req.folder,
        since: None,
        is_unread: req.is_unread,
        limit: req.limit,
    };

    let results = mail
        .search(query)
        .await
        .map_err(|_err| StatusCode::SERVICE_UNAVAILABLE)?;

    let count = results.len();
    Ok(Json(SearchResponse { results, count }))
}
