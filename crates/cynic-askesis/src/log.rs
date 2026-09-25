//! Askesis log — append-only chain of content-addressed entries.

use serde::{Deserialize, Serialize};

use crate::entry::{AskesisEntry, AskesisKind, GENESIS_HASH};

pub const ASKESIS_SCHEMA: &str = "cynic.askesis.v1";

/// Askesis log — append-only chain of entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskesisLog {
    /// Schema version
    pub schema: String,
    /// Log identity (which log is this)
    pub log_id: String,
    /// All entries in order
    pub entries: Vec<AskesisEntry>,
}

impl AskesisLog {
    pub fn new(log_id: impl Into<String>) -> Self {
        Self {
            schema: ASKESIS_SCHEMA.to_string(),
            log_id: log_id.into(),
            entries: Vec::new(),
        }
    }

    /// Append an entry with auto-chaining.
    pub fn append(
        &mut self,
        kind: AskesisKind,
        observed_at_unix_s: u64,
        subject: impl Into<String>,
        evidence: serde_json::Value,
    ) -> &AskesisEntry {
        let sequence = self.entries.len() as u64 + 1;
        let prev_hash = self
            .entries
            .last()
            .map(|e| e.entry_hash.clone())
            .unwrap_or_else(|| GENESIS_HASH.to_string());

        let entry = AskesisEntry::new(
            kind,
            observed_at_unix_s,
            sequence,
            prev_hash,
            subject,
            evidence,
        );
        self.entries.push(entry);
        self.entries.last().unwrap()
    }

    /// Verify entire chain integrity.
    pub fn verify_chain(&self) -> bool {
        for (i, entry) in self.entries.iter().enumerate() {
            if !entry.verify() {
                return false;
            }
            if i == 0 {
                if entry.prev_hash != GENESIS_HASH {
                    return false;
                }
            } else if entry.prev_hash != self.entries[i - 1].entry_hash {
                return false;
            }
        }
        true
    }

    pub fn last_hash(&self) -> Option<&str> {
        self.entries.last().map(|e| e.entry_hash.as_str())
    }
}
