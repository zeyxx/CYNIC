//! JSONL-backed LogStore implementation.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::log::{LogStore, ObservationEvent};

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
}

impl LogStore for JsonlLog {
    fn append(&mut self, mut entry: ObservationEvent) -> crate::Result<()> {
        use sha2::{Digest, Sha512};

        // 1. Fetch the prev_hash
        let prev_hash = self.get_last_hash().unwrap_or_else(|| "0".repeat(128));
        entry.prev_hash = Some(prev_hash.clone());

        // 2. Compute the new cryptographic_hash (SHA512 of payload + prev_hash)
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
        let hash_hex = hash_result
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

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

        for line in reader.lines().flatten() {
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
