//! Port for durable chat session event store (principal-owned).

use crate::domain::chat_session::{ChatEvent, SessionMeta};

#[derive(Debug, Clone, thiserror::Error)]
pub enum SessionStoreError {
    #[error("session not found: {0}")]
    NotFound(String),
    #[error("io: {0}")]
    Io(String),
    #[error("protocol: {0}")]
    Protocol(String),
}

/// Append-only session transcript store.
pub trait ChatSessionStore: Send + Sync {
    fn create_session(
        &self,
        id: &str,
        title: Option<&str>,
        model: Option<&str>,
        endpoint: Option<&str>,
    ) -> Result<SessionMeta, SessionStoreError>;

    fn get_meta(&self, id: &str) -> Result<SessionMeta, SessionStoreError>;

    fn list_sessions(&self) -> Result<Vec<SessionMeta>, SessionStoreError>;

    fn append(&self, id: &str, event: ChatEvent) -> Result<(), SessionStoreError>;

    fn load_events(&self, id: &str) -> Result<Vec<ChatEvent>, SessionStoreError>;

    /// Export all events as JSONL string.
    fn export_jsonl(&self, id: &str) -> Result<String, SessionStoreError>;

    fn root_dir(&self) -> &std::path::Path;
}
