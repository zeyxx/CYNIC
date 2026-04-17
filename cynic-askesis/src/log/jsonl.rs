//! JSONL-backed LogStore implementation.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::log::{LogEntry, LogStore};

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
}

impl LogStore for JsonlLog {
    fn append(&mut self, entry: LogEntry) -> crate::Result<()> {
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        let line = serde_json::to_string(&entry)?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    fn range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> crate::Result<Vec<LogEntry>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut out = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: LogEntry = serde_json::from_str(&line)?;
            if entry.timestamp >= from && entry.timestamp <= to {
                out.push(entry);
            }
        }
        Ok(out)
    }
}
