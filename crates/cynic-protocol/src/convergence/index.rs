use serde::{Deserialize, Serialize};

use super::causal::CAUSAL_CONVERGENCE_EPOCH_SCHEMA;
use crate::evidence::valid_sha256_digest;

pub const CAUSAL_EPOCH_INDEX_SCHEMA: &str = "cynic.convergence_epoch_index.v1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CausalEpochKey {
    pub subject_id: String,
    pub vertical_id: String,
    pub epoch_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalEpochIndexEntry {
    pub key: CausalEpochKey,
    pub epoch_digest: String,
    pub observed_at_unix_ms: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CausalEpochIndexAppend {
    Inserted,
    AlreadyPresent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalEpochIndex {
    pub schema: String,
    pub epoch_schema: String,
    #[serde(default)]
    pub entries: Vec<CausalEpochIndexEntry>,
}

impl Default for CausalEpochIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl CausalEpochIndex {
    pub fn new() -> Self {
        Self {
            schema: CAUSAL_EPOCH_INDEX_SCHEMA.to_string(),
            epoch_schema: CAUSAL_CONVERGENCE_EPOCH_SCHEMA.to_string(),
            entries: Vec::new(),
        }
    }

    pub fn from_entries(entries: Vec<CausalEpochIndexEntry>) -> Result<Self, String> {
        let mut index = Self {
            entries,
            ..Self::new()
        };
        index
            .entries
            .sort_by(|left, right| left.key.cmp(&right.key));
        index.verify()?;
        Ok(index)
    }

    /// Appends one coordinate-to-digest binding. Replaying the exact binding
    /// is idempotent; rebinding a coordinate is a conflict, never an update.
    pub fn append(
        &mut self,
        entry: CausalEpochIndexEntry,
    ) -> Result<CausalEpochIndexAppend, String> {
        validate_entry(&entry)?;
        self.verify()?;
        match self
            .entries
            .binary_search_by(|candidate| candidate.key.cmp(&entry.key))
        {
            Ok(position) => {
                if self.entries[position] == entry {
                    Ok(CausalEpochIndexAppend::AlreadyPresent)
                } else {
                    Err("causal_epoch_index_coordinate_conflict".to_string())
                }
            }
            Err(position) => {
                self.entries.insert(position, entry);
                Ok(CausalEpochIndexAppend::Inserted)
            }
        }
    }

    pub fn get(&self, key: &CausalEpochKey) -> Option<&CausalEpochIndexEntry> {
        self.entries
            .binary_search_by(|candidate| candidate.key.cmp(key))
            .ok()
            .map(|position| &self.entries[position])
    }

    /// Returns the newest epoch for one subject and vertical. Ties are resolved
    /// by epoch id to keep projections deterministic without mutating history.
    pub fn latest_for(
        &self,
        subject_id: &str,
        vertical_id: &str,
    ) -> Option<&CausalEpochIndexEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                entry.key.subject_id == subject_id && entry.key.vertical_id == vertical_id
            })
            .max_by(|left, right| {
                left.observed_at_unix_ms
                    .cmp(&right.observed_at_unix_ms)
                    .then_with(|| left.key.epoch_id.cmp(&right.key.epoch_id))
            })
    }

    pub fn verify(&self) -> Result<(), String> {
        if self.schema != CAUSAL_EPOCH_INDEX_SCHEMA {
            return Err("causal_epoch_index_schema_mismatch".to_string());
        }
        if self.epoch_schema != CAUSAL_CONVERGENCE_EPOCH_SCHEMA {
            return Err("causal_epoch_index_epoch_schema_mismatch".to_string());
        }
        for (position, entry) in self.entries.iter().enumerate() {
            validate_entry(entry)?;
            if position > 0 {
                let previous = &self.entries[position - 1];
                if previous.key >= entry.key {
                    return Err("causal_epoch_index_entries_not_canonical".to_string());
                }
            }
        }
        Ok(())
    }
}

fn validate_entry(entry: &CausalEpochIndexEntry) -> Result<(), String> {
    if entry.key.subject_id.trim().is_empty()
        || entry.key.vertical_id.trim().is_empty()
        || entry.key.epoch_id.trim().is_empty()
    {
        return Err("causal_epoch_index_coordinate_missing".to_string());
    }
    if entry.observed_at_unix_ms == 0 {
        return Err("causal_epoch_index_observed_at_missing".to_string());
    }
    if !valid_sha256_digest(&entry.epoch_digest) {
        return Err("causal_epoch_index_digest_invalid".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(marker: char) -> String {
        format!("sha256:{}", marker.to_string().repeat(64))
    }

    fn entry(epoch_id: &str, marker: char, observed_at_unix_ms: u128) -> CausalEpochIndexEntry {
        CausalEpochIndexEntry {
            key: CausalEpochKey {
                subject_id: "local-kernel".to_string(),
                vertical_id: "process".to_string(),
                epoch_id: epoch_id.to_string(),
            },
            epoch_digest: digest(marker),
            observed_at_unix_ms,
        }
    }

    #[test]
    fn exact_epoch_binding_is_idempotent() {
        let mut index = CausalEpochIndex::new();
        let binding = entry("epoch-1", '1', 1);

        assert_eq!(
            index.append(binding.clone()).unwrap(),
            CausalEpochIndexAppend::Inserted
        );
        assert_eq!(
            index.append(binding).unwrap(),
            CausalEpochIndexAppend::AlreadyPresent
        );
        assert_eq!(index.entries.len(), 1);
    }

    #[test]
    fn coordinate_rebinding_is_blocked() {
        let mut index = CausalEpochIndex::new();
        index.append(entry("epoch-1", '1', 1)).unwrap();

        let error = index.append(entry("epoch-1", '2', 1)).unwrap_err();
        assert_eq!(error, "causal_epoch_index_coordinate_conflict");
    }

    #[test]
    fn latest_projection_is_deterministic_and_does_not_rewrite_history() {
        let mut index = CausalEpochIndex::new();
        index.append(entry("epoch-1", '1', 1)).unwrap();
        index.append(entry("epoch-2", '2', 2)).unwrap();

        let latest = index.latest_for("local-kernel", "process").unwrap();
        assert_eq!(latest.key.epoch_id, "epoch-2");
        assert_eq!(index.entries.len(), 2);
    }

    #[test]
    fn invalid_deserialized_index_is_rejected() {
        let mut index = CausalEpochIndex::new();
        index.entries = vec![entry("epoch-1", '1', 1), entry("epoch-1", '2', 2)];

        assert_eq!(
            index.verify().unwrap_err(),
            "causal_epoch_index_entries_not_canonical"
        );
    }
}
