//! LogStore trait + ObservationEvent — free-form text logging.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::Result;

/// A single free-form log entry.
///
/// `domain` is optional — Phase 1 accepts free-form logs without domain
/// classification. Gemini audit detects dimensions from the text itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservationEvent {
    pub timestamp: i64, // Unix Epoch in milliseconds
    pub domain: Option<String>,
    pub content: String,

    // Cryptographic Envelope (Proof-of-History Stage 2 -> Stage 6)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cryptographic_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_hash: Option<String>,
}

impl ObservationEvent {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now().timestamp_millis(),
            domain: None,
            content: content.into(),
            cryptographic_hash: None,
            prev_hash: None,
        }
    }

    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }
}

/// Persistence port for log entries.
pub trait LogStore {
    fn append(&mut self, entry: ObservationEvent) -> Result<()>;
    fn range(&self, from: i64, to: i64) -> Result<Vec<ObservationEvent>>;
}

pub mod jsonl;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_entry_new_sets_content_and_timestamp() {
        let entry = ObservationEvent::new("pas fait de sport");
        assert_eq!(entry.content, "pas fait de sport");
        assert!(entry.domain.is_none());
        assert!(entry.timestamp <= Utc::now().timestamp_millis());
    }

    #[test]
    fn log_entry_with_domain_sets_domain() {
        let entry = ObservationEvent::new("pompes 3x20").with_domain("body");
        assert_eq!(entry.domain.as_deref(), Some("body"));
    }

    #[test]
    fn log_entry_serializes_roundtrip() {
        let entry = ObservationEvent::new("test").with_domain("body");
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: ObservationEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, parsed);
    }
}
