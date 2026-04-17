//! LogStore trait + LogEntry — free-form text logging.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Result;

/// A single free-form log entry.
///
/// `domain` is optional — Phase 1 accepts free-form logs without domain
/// classification. Gemini audit detects dimensions from the text itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub domain: Option<String>,
    pub content: String,
}

impl LogEntry {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            domain: None,
            content: content.into(),
        }
    }

    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }
}

/// Persistence port for log entries.
pub trait LogStore {
    fn append(&mut self, entry: LogEntry) -> Result<()>;
    fn range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<LogEntry>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_entry_new_sets_content_and_timestamp() {
        let entry = LogEntry::new("pas fait de sport");
        assert_eq!(entry.content, "pas fait de sport");
        assert!(entry.domain.is_none());
        assert!(entry.timestamp <= Utc::now());
    }

    #[test]
    fn log_entry_with_domain_sets_domain() {
        let entry = LogEntry::new("pompes 3x20").with_domain("body");
        assert_eq!(entry.domain.as_deref(), Some("body"));
    }

    #[test]
    fn log_entry_serializes_roundtrip() {
        let entry = LogEntry::new("test").with_domain("body");
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: LogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, parsed);
    }
}
