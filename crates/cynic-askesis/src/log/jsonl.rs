//! JSONL-backed LogStore implementation with retention, compaction, and PoH chain verification.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

use crate::log::{LogStore, ObservationEvent};

/// Computes the deterministic SHA-512 Proof-of-History hash for an event chained to `prev_hash`.
pub fn compute_event_hash(entry: &ObservationEvent, prev_hash: &str) -> String {
    let payload = format!(
        "{}_{}_{}",
        entry.timestamp,
        entry.domain.as_deref().unwrap_or(""),
        entry.content
    );
    let mut hasher = Sha512::new();
    hasher.update(payload.as_bytes());
    hasher.update(prev_hash.as_bytes());
    let hash_result = hasher.finalize();
    hash_result
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogRetentionPolicy {
    pub max_active_entries: usize,
    pub max_age_ms: Option<i64>,
    pub archive_dir: Option<PathBuf>,
}

impl Default for LogRetentionPolicy {
    fn default() -> Self {
        Self {
            max_active_entries: 1000,
            max_age_ms: None,
            archive_dir: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionReport {
    pub total_before: usize,
    pub active_count: usize,
    pub archived_count: usize,
    pub anchor_hash: Option<String>,
    pub archive_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainVerificationResult {
    pub valid_entries: usize,
    pub is_valid: bool,
    pub head_hash: Option<String>,
    pub violations: Vec<String>,
}

#[derive(Debug)]
pub struct JsonlLog {
    path: PathBuf,
}

impl JsonlLog {
    /// Opens (creating if needed) a JSONL log at `path`.
    ///
    /// Parent directory is created if missing.
    pub fn new(path: impl Into<PathBuf>) -> crate::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Touch file to ensure existence for range() when empty.
        OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Reads all entries from the log file.
    pub fn read_all(&self) -> crate::Result<Vec<ObservationEvent>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines().map_while(Result::ok) {
            if line.trim().is_empty() {
                continue;
            }
            let entry: ObservationEvent = serde_json::from_str(&line)?;
            entries.push(entry);
        }
        Ok(entries)
    }

    fn get_last_hash(&self) -> Option<String> {
        let file = File::open(&self.path).ok()?;
        let reader = BufReader::new(file);
        let mut last_hash = None;
        for line in reader.lines().map_while(Result::ok) {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<ObservationEvent>(&line)
                && let Some(h) = entry.cryptographic_hash
            {
                last_hash = Some(h);
            }
        }
        last_hash
    }

    /// Verifies the cryptographic Proof-of-History hash chain integrity.
    pub fn verify_chain(&self) -> crate::Result<ChainVerificationResult> {
        let entries = self.read_all()?;
        let mut violations = Vec::new();
        let mut expected_prev_hash: Option<String> = None;
        let mut head_hash = None;

        for (i, entry) in entries.iter().enumerate() {
            let declared_prev = entry.prev_hash.as_deref().unwrap_or("");
            if let Some(expected) = &expected_prev_hash {
                if declared_prev != expected {
                    violations.push(format!(
                        "entry_{i}_prev_hash_mismatch: declared {declared_prev} vs expected {expected}"
                    ));
                }
            }

            let expected_current = compute_event_hash(entry, declared_prev);
            if let Some(declared_hash) = &entry.cryptographic_hash {
                if declared_hash != &expected_current {
                    violations.push(format!(
                        "entry_{i}_hash_mismatch: declared {declared_hash} vs computed {expected_current}"
                    ));
                }
                expected_prev_hash = Some(declared_hash.clone());
                head_hash = Some(declared_hash.clone());
            } else {
                violations.push(format!("entry_{i}_cryptographic_hash_missing"));
                expected_prev_hash = Some(expected_current);
            }
        }

        Ok(ChainVerificationResult {
            valid_entries: entries.len().saturating_sub(violations.len()),
            is_valid: violations.is_empty(),
            head_hash,
            violations,
        })
    }

    /// Compacts log entries according to the retention policy without breaking the PoH chain.
    pub fn compact(&mut self, policy: &LogRetentionPolicy) -> crate::Result<CompactionReport> {
        let entries = self.read_all()?;
        let total_before = entries.len();
        if total_before == 0 {
            return Ok(CompactionReport {
                total_before: 0,
                active_count: 0,
                archived_count: 0,
                anchor_hash: None,
                archive_path: None,
            });
        }

        // 1. Verify chain integrity before any compaction
        let verification = self.verify_chain()?;
        if !verification.is_valid {
            return Err(crate::error::AskesisError::InvalidObservationEvent(
                format!(
                    "cannot compact corrupted log chain: {:?}",
                    verification.violations
                ),
            ));
        }

        // 2. Identify active cutoff
        let cutoff_index = if entries.len() > policy.max_active_entries {
            entries.len() - policy.max_active_entries
        } else {
            0
        };

        let (archived, active) = entries.split_at(cutoff_index);
        if archived.is_empty() {
            return Ok(CompactionReport {
                total_before,
                active_count: active.len(),
                archived_count: 0,
                anchor_hash: active.first().and_then(|e| e.prev_hash.clone()),
                archive_path: None,
            });
        }

        // 3. Write archive if directory configured
        let archive_path = if let Some(archive_dir) = &policy.archive_dir {
            std::fs::create_dir_all(archive_dir)?;
            let first_ts = archived.first().map(|e| e.timestamp).unwrap_or(0);
            let last_ts = archived.last().map(|e| e.timestamp).unwrap_or(0);
            let file_name = format!("archive-{first_ts}-{last_ts}.jsonl");
            let path = archive_dir.join(file_name);
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)?;
            for entry in archived {
                let line = serde_json::to_string(entry)?;
                writeln!(file, "{line}")?;
            }
            Some(path)
        } else {
            None
        };

        // 4. Atomically rewrite active log file
        let temp_path = self.path.with_extension("tmp");
        {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&temp_path)?;
            for entry in active {
                let line = serde_json::to_string(entry)?;
                writeln!(file, "{line}")?;
            }
            file.flush()?;
        }
        std::fs::rename(&temp_path, &self.path)?;

        let anchor_hash = archived.last().and_then(|e| e.cryptographic_hash.clone());

        Ok(CompactionReport {
            total_before,
            active_count: active.len(),
            archived_count: archived.len(),
            anchor_hash,
            archive_path,
        })
    }
}

impl LogStore for JsonlLog {
    fn append(&mut self, mut entry: ObservationEvent) -> crate::Result<()> {
        // 1. Fetch the prev_hash
        let prev_hash = self.get_last_hash().unwrap_or_else(|| "0".repeat(128));
        entry.prev_hash = Some(prev_hash.clone());

        // 2. Compute the new cryptographic_hash (SHA512 of payload + prev_hash)
        let hash_hex = compute_event_hash(&entry, &prev_hash);
        entry.cryptographic_hash = Some(hash_hex);

        // 3. Write to the append-only JSONL
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        let line = serde_json::to_string(&entry)?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    fn range(&self, from: i64, to: i64) -> crate::Result<Vec<ObservationEvent>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut results = Vec::new();

        for line in reader.lines().map_while(Result::ok) {
            if line.trim().is_empty() {
                continue;
            }
            let entry: ObservationEvent = serde_json::from_str(&line)?;
            if entry.timestamp >= from && entry.timestamp <= to {
                results.push(entry);
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poh_chain_verification_and_compaction() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let log_path = temp_dir.path().join("test-log.jsonl");
        let archive_dir = temp_dir.path().join("archives");

        let mut log = JsonlLog::new(&log_path).expect("new log");

        // Append 5 events
        for i in 1..=5 {
            let event = ObservationEvent::new(format!("event {i}")).with_domain("test");
            log.append(event).expect("append");
        }

        // Verify chain integrity
        let verification = log.verify_chain().expect("verify");
        assert!(verification.is_valid);
        assert_eq!(verification.valid_entries, 5);

        // Compact keeping 2 active entries
        let policy = LogRetentionPolicy {
            max_active_entries: 2,
            max_age_ms: None,
            archive_dir: Some(archive_dir),
        };

        let report = log.compact(&policy).expect("compact");
        assert_eq!(report.total_before, 5);
        assert_eq!(report.active_count, 2);
        assert_eq!(report.archived_count, 3);
        assert!(report.archive_path.is_some());
        assert!(report.anchor_hash.is_some());

        // Read active entries: must have 2
        let active_entries = log.read_all().expect("read active");
        assert_eq!(active_entries.len(), 2);
        assert_eq!(active_entries[0].content, "event 4");
        assert_eq!(active_entries[1].content, "event 5");

        // The first active entry's prev_hash must equal the anchor_hash
        assert_eq!(
            active_entries[0].prev_hash.as_ref(),
            report.anchor_hash.as_ref()
        );
    }
}
