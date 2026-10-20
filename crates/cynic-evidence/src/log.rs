//! Evidence log — append-only chain of content-addressed entries.

use serde::{Deserialize, Serialize};

use crate::entry::{EvidenceEntry, EvidenceKind, GENESIS_HASH};

pub const EVIDENCE_SCHEMA: &str = "cynic.evidence.v1";

/// Evidence log — append-only chain of entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceLog {
    /// Schema version
    pub schema: String,
    /// Log identity (which log is this)
    pub log_id: String,
    /// All entries in order
    pub entries: Vec<EvidenceEntry>,
}

impl EvidenceLog {
    pub fn new(log_id: impl Into<String>) -> Self {
        Self {
            schema: EVIDENCE_SCHEMA.to_string(),
            log_id: log_id.into(),
            entries: Vec::new(),
        }
    }

    /// Append an entry with auto-chaining.
    pub fn append(
        &mut self,
        kind: EvidenceKind,
        observed_at_unix_s: u64,
        subject: impl Into<String>,
        evidence: serde_json::Value,
    ) -> &EvidenceEntry {
        let sequence = self.entries.len() as u64 + 1;
        let prev_hash = self
            .entries
            .last()
            .map(|e| e.entry_hash.clone())
            .unwrap_or_else(|| GENESIS_HASH.to_string());

        let entry = EvidenceEntry::new(
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
