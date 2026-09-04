//! Append-only WAL file management with fsync durability and hash-chain verification.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use cynic_protocol::peer::{GENESIS_PREV_HASH, WalEntry, verify_wal_chain};

#[derive(Debug)]
pub struct PeerWalFile {
    path: PathBuf,
    last_hash: String,
    last_sequence: u64,
}

impl PeerWalFile {
    /// Open or create the WAL file, reading and verifying existing chain integrity.
    pub fn open(path: impl AsRef<Path>) -> Result<(Self, Vec<WalEntry>), String> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("failed to create dir: {e}"))?;
        }

        let mut entries = Vec::new();
        if path.exists() {
            let file = File::open(&path).map_err(|e| format!("failed to open WAL file: {e}"))?;
            let reader = BufReader::new(file);
            for (line_num, line) in reader.lines().enumerate() {
                let line = line.map_err(|e| format!("read error line {line_num}: {e}"))?;
                if line.trim().is_empty() {
                    continue;
                }
                let entry: WalEntry = serde_json::from_str(&line)
                    .map_err(|e| format!("corrupted JSON line {line_num}: {e}"))?;
                entries.push(entry);
            }
        }

        // Verify chain integrity from genesis
        verify_wal_chain(&entries, GENESIS_PREV_HASH)
            .map_err(|e| format!("WAL integrity verification failed: {e}"))?;

        let (last_hash, last_sequence) = if let Some(last) = entries.last() {
            (last.entry_hash.clone(), last.sequence)
        } else {
            (GENESIS_PREV_HASH.to_string(), 0)
        };

        Ok((
            Self {
                path,
                last_hash,
                last_sequence,
            },
            entries,
        ))
    }

    /// Append a single sealed entry to the WAL file with immediate sync.
    pub fn append(&mut self, entry: &WalEntry) -> Result<(), String> {
        if entry.sequence != self.last_sequence + 1 {
            return Err(format!(
                "sequence mismatch: expected {}, got {}",
                self.last_sequence + 1,
                entry.sequence
            ));
        }
        if entry.prev_hash != self.last_hash {
            return Err(format!(
                "prev_hash mismatch: expected {}, got {}",
                self.last_hash, entry.prev_hash
            ));
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("failed to open WAL for append: {e}"))?;

        let line = serde_json::to_string(entry)
            .map_err(|e| format!("failed to serialize WAL entry: {e}"))?;

        writeln!(file, "{line}").map_err(|e| format!("failed to write to WAL: {e}"))?;
        file.sync_data()
            .map_err(|e| format!("failed to sync WAL to disk: {e}"))?;

        self.last_hash = entry.entry_hash.clone();
        self.last_sequence = entry.sequence;
        Ok(())
    }

    pub fn last_hash(&self) -> &str {
        &self.last_hash
    }

    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }
}
