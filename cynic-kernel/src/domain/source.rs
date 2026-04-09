//! ObservationSource v1 — domain contract for inbound observation feeds.
//!
//! An observation source is a long-running adapter that connects to an external
//! system (Telegram, X, Slack, Discord, GitHub events, on-chain stream, etc.),
//! listens for relevant events, and emits them into the kernel's observation
//! pipeline (`StoragePort::store_observation`).
//!
//! This trait absorbs patterns from prior art researched 2026-04-09:
//! - vtable Channel + ring-buffer bus (nullclaw, Zig)
//! - source-as-async-function + delta engine (Crucix, JS)
//! - Adapter interface + Message/Thread abstraction (vercel/chat, TS)
//! - dual-interface boundary + context-as-callback-bundle (pi-mono, TS)
//!
//! Sources can be read-only (X observer scrape) or bidirectional. Bidirectional
//! sources implement the `OutboundSource` extension trait below to declare a
//! `send` capability.
//!
//! Lifecycle: spawned at kernel boot via `infra::tasks::spawn_observation_sources`,
//! supervised with exponential backoff on failure (see `sources::supervisor`),
//! gracefully shut down via `CancellationToken`.
//!
//! Security: every source that holds credentials declares its credential path
//! via `credential_path()`. Credentials are loaded via `CredentialPort` (see
//! `domain::credential`), which enforces encrypted-at-rest semantics. Sources
//! check `is_paused()` before any side-effect operation, allowing the user to
//! emergency-stop a source by touching a kill-switch file.
//!
//! K10: agents/sources delegate persistence to the kernel. Sources never own
//! their own DB; events flow through `store_observation`.
//! K15: every source event has an acting consumer — the storage layer + the
//! pipeline embedding step + crystal eligibility, all chained automatically.

use crate::domain::storage::Observation;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

/// A discrete event emitted by an observation source. The kernel converts each
/// event into an `Observation` via `to_observation()` and stores it.
///
/// Typed (no `serde_json::Value`) per Gate 3: domain stays adapter-agnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEvent {
    /// Stable identifier of the source instance that emitted this event.
    /// Format: "{kind}:{instance_id}" — e.g. "telegram_user:zey", "x_observer:gcrtrd".
    pub source_id: String,

    /// What kind of event this is. Source-specific vocabulary, but should be
    /// stable for crystal-formation purposes (events with the same `kind` form
    /// the same crystal class).
    /// Examples: "message_received", "message_sent", "tweet_seen", "user_followed".
    pub kind: String,

    /// Optional target identifier — chat_id, user handle, channel name, etc.
    /// Used for routing, filtering, and observation `target` field.
    pub target: Option<String>,

    /// The substantive content of the event — message text, tweet body, etc.
    /// This is what gets embedded for crystal search.
    pub content: String,

    /// Source-of-truth timestamp from the external system, ISO-8601.
    /// Sources are responsible for normalizing to UTC.
    pub external_timestamp: String,

    /// Free-form structured metadata as a list of key-value pairs.
    /// Typed as `Vec<(String, String)>` instead of `serde_json::Value` to honor
    /// Gate 3 (zero serde_json::Value in domain/). Adapters serialize structured
    /// data into stringified entries (e.g. ("stats.likes", "42")) for storage.
    pub metadata: Vec<(String, String)>,
}

impl SourceEvent {
    /// Convert this event into the kernel's `Observation` format for storage.
    /// The mapping is intentionally lossy on `metadata` — only the most useful
    /// fields are flattened into the observation's `context` string.
    /// Full structured data is preserved by the source's own audit log if needed.
    pub fn to_observation(&self, agent_id: &str, project: &str) -> Observation {
        let metadata_summary = if self.metadata.is_empty() {
            String::new()
        } else {
            let parts: Vec<String> = self
                .metadata
                .iter()
                .take(8) // bounded to avoid bloating context
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            format!(" | {}", parts.join(", "))
        };

        Observation {
            project: project.to_string(),
            agent_id: agent_id.to_string(),
            tool: format!("source:{}", self.kind),
            target: self.target.clone().unwrap_or_default(),
            domain: "general".to_string(),
            status: "observed".to_string(),
            context: format!("{}{}", self.content, metadata_summary),
            session_id: self.source_id.clone(),
            timestamp: self.external_timestamp.clone(),
        }
    }
}

/// Health of an observation source — exposed via `/health` for /metrics surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceHealth {
    /// Connected and producing events normally.
    Healthy,
    /// Connected but no events for an unusually long time. Could be normal
    /// (slow channel) or could be a silent failure. The supervisor decides.
    Idle,
    /// Disconnected, attempting reconnect with backoff.
    Reconnecting { attempt: u32, last_error: String },
    /// Permanently failed (e.g. auth revoked, account banned). Manual recovery required.
    Failed { reason: String },
    /// User-paused via kill switch file. Source is alive but refuses to act.
    Paused,
}

/// Errors raised by observation sources.
#[derive(Debug, Error)]
pub enum SourceError {
    #[error("Source connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Source authentication failed: {0}")]
    AuthFailed(String),
    #[error("Credential not found at {path:?}: {reason}")]
    CredentialMissing { path: PathBuf, reason: String },
    #[error("Source rate-limited, retry after {retry_after_secs}s: {context}")]
    RateLimited {
        retry_after_secs: u64,
        context: String,
    },
    #[error("Source paused via kill-switch file at {0:?}")]
    Paused(PathBuf),
    #[error("Source target not in whitelist: {0}")]
    TargetNotWhitelisted(String),
    #[error("Source operation timed out after {0}s")]
    Timeout(u64),
    #[error("Source protocol error: {0}")]
    ProtocolError(String),
    #[error("Source IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Source generic error: {0}")]
    Other(String),
}

/// Async function type for emitting events from a source's `run()` loop into
/// the kernel observation pipeline. The kernel provides an instance of this
/// closure to `run()`; sources call it for each event they observe.
///
/// Boxed + Send + Sync so it can be passed across `tokio::spawn` boundaries.
pub type SourceEmitter = std::sync::Arc<
    dyn Fn(
            SourceEvent,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), SourceError>> + Send>>
        + Send
        + Sync,
>;

/// Core trait for an inbound observation source.
///
/// Implementors are spawned at kernel boot, supervised with exponential backoff,
/// and emit `SourceEvent`s into the observation pipeline via the `run()` callback.
#[async_trait]
pub trait ObservationSource: Send + Sync {
    /// Stable instance identifier for this source.
    /// Format suggestion: lowercase, snake_case, short. Example: "zey".
    fn id(&self) -> &str;

    /// What KIND of source this is — used for routing, filtering, and the
    /// `tool` field of stored observations. Example: "telegram_user", "x_observer".
    fn kind(&self) -> &str;

    /// Path to this source's credential file, if any.
    /// Used by the supervisor to verify file permissions at startup (chmod 600
    /// expected) and by the security audit log to flag credential access.
    /// Read-only sources without credentials return `None`.
    fn credential_path(&self) -> Option<PathBuf>;

    /// Path to the kill-switch file for this source.
    /// Default: `~/.local/share/cynic/sources/{id}.PAUSE`. If this file exists,
    /// the source MUST refuse all side-effect operations and report `Paused`
    /// health. The user can emergency-stop a source by `touch`-ing this file.
    fn kill_switch_path(&self) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(format!(
            "{home}/.local/share/cynic/sources/{}.PAUSE",
            self.id()
        ))
    }

    /// Check whether this source is currently paused via its kill-switch file.
    /// Default impl checks file existence; override if a source needs more
    /// elaborate pause logic (e.g. time-windowed pauses).
    fn is_paused(&self) -> bool {
        self.kill_switch_path().exists()
    }

    /// Connect to the external service. Idempotent — if already connected,
    /// returns Ok without side effects. Called by the supervisor at boot and
    /// after each backoff cycle.
    async fn connect(&mut self) -> Result<(), SourceError>;

    /// Disconnect cleanly from the external service. Idempotent. Called on
    /// graceful shutdown and before the supervisor backs off after a failure.
    async fn disconnect(&mut self) -> Result<(), SourceError>;

    /// Long-running event loop. The source listens for external events and
    /// calls `emit(event)` for each one. The loop terminates when:
    /// - `shutdown` is cancelled (graceful), OR
    /// - the source returns an error (supervisor will retry with backoff).
    ///
    /// Sources MUST honor the `is_paused()` check before emitting any event.
    /// Sources SHOULD report progress via `task_health.touch_X()` if they have
    /// a health hook registered (managed by the supervisor).
    async fn run(
        &mut self,
        emit: SourceEmitter,
        shutdown: CancellationToken,
    ) -> Result<(), SourceError>;

    /// Current health snapshot — used by `/health` and the supervisor's
    /// stale-detection logic. Should not block.
    fn health(&self) -> SourceHealth;
}

/// Extension trait for sources that can SEND messages back to the external
/// system (Telegram DM, Slack post, Discord message, etc.).
///
/// Read-only sources (X observer scrape, GitHub event ingestion) only implement
/// `ObservationSource`, not this trait. The kernel checks at registration time
/// whether a source is `OutboundSource` to decide which MCP tools to expose.
#[async_trait]
pub trait OutboundSource: ObservationSource {
    /// Send a message to a target on the external system.
    ///
    /// `target` is a source-specific identifier (chat_id, user handle, channel id).
    /// `content` is the message body, plain text by default.
    ///
    /// Implementations MUST:
    /// - Check `is_paused()` and return `SourceError::Paused` if true.
    /// - Check `target` against the source's whitelist (if any) and return
    ///   `SourceError::TargetNotWhitelisted` if not allowed.
    /// - Honor rate limits and return `SourceError::RateLimited` rather than
    ///   blocking the caller indefinitely.
    /// - Write an entry to the source's audit log before sending.
    ///
    /// Returns a `MessageRef` that the caller can use to track delivery / edit
    /// the message later.
    async fn send(&mut self, target: &str, content: &str) -> Result<MessageRef, SourceError>;
}

/// Reference to a message sent by an `OutboundSource`. Returned by `send()` so
/// callers can later edit, delete, or react to the message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRef {
    /// External system's message ID (e.g. Telegram message_id, Slack ts).
    pub external_id: String,
    /// Target this message was sent to.
    pub target: String,
    /// UTC ISO-8601 timestamp when the kernel dispatched the send.
    pub sent_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_event_to_observation_minimal() {
        let event = SourceEvent {
            source_id: "telegram_user:zey".to_string(),
            kind: "message_received".to_string(),
            target: Some("stanislaz".to_string()),
            content: "salut".to_string(),
            external_timestamp: "2026-04-09T14:00:00Z".to_string(),
            metadata: vec![],
        };
        let obs = event.to_observation("zey", "cynic-hackathon-observatory");
        assert_eq!(obs.tool, "source:message_received");
        assert_eq!(obs.target, "stanislaz");
        assert_eq!(obs.context, "salut");
        assert_eq!(obs.session_id, "telegram_user:zey");
        assert_eq!(obs.timestamp, "2026-04-09T14:00:00Z");
    }

    #[test]
    fn source_event_to_observation_with_metadata() {
        let event = SourceEvent {
            source_id: "x_observer:gcrtrd".to_string(),
            kind: "tweet_seen".to_string(),
            target: Some("gcrtrd".to_string()),
            content: "ASDF maximalist".to_string(),
            external_timestamp: "2026-04-09T14:00:00Z".to_string(),
            metadata: vec![
                ("likes".to_string(), "42".to_string()),
                ("retweets".to_string(), "11".to_string()),
            ],
        };
        let obs = event.to_observation("zey", "cynic-hackathon-observatory");
        assert!(obs.context.contains("ASDF maximalist"));
        assert!(obs.context.contains("likes=42"));
        assert!(obs.context.contains("retweets=11"));
    }

    #[test]
    fn source_event_target_optional() {
        let event = SourceEvent {
            source_id: "x_observer:home".to_string(),
            kind: "timeline_refresh".to_string(),
            target: None,
            content: "12 new tweets".to_string(),
            external_timestamp: "2026-04-09T14:00:00Z".to_string(),
            metadata: vec![],
        };
        let obs = event.to_observation("zey", "cynic-hackathon-observatory");
        assert_eq!(obs.target, ""); // None becomes empty string
    }

    #[test]
    fn metadata_bounded_to_8_entries() {
        let metadata: Vec<(String, String)> = (0..20)
            .map(|i| (format!("k{i}"), format!("v{i}")))
            .collect();
        let event = SourceEvent {
            source_id: "test:bounded".to_string(),
            kind: "test".to_string(),
            target: None,
            content: "x".to_string(),
            external_timestamp: "2026-04-09T14:00:00Z".to_string(),
            metadata,
        };
        let obs = event.to_observation("a", "p");
        // Only first 8 metadata entries should appear
        assert!(obs.context.contains("k0=v0"));
        assert!(obs.context.contains("k7=v7"));
        assert!(!obs.context.contains("k8=v8"));
        assert!(!obs.context.contains("k19=v19"));
    }
}
