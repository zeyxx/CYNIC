use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use cynic_protocol::HealthSignal;

use crate::ports::observation_journal_port::ObservationJournalPort;

/// Local JSONL append-only Write-Ahead Log (WAL) adapter.
#[derive(Debug, Clone)]
pub struct LocalObservationJournal {
    path: PathBuf,
}

impl LocalObservationJournal {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_path() -> PathBuf {
        PathBuf::from("/var/lib/cynic/journal.jsonl")
    }
}

impl ObservationJournalPort for LocalObservationJournal {
    fn append_signal(
        &self,
        signal: &HealthSignal,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(signal)?;
        writeln!(file, "{line}")?;
        file.sync_data()?;
        Ok(())
    }

    fn read_signals(
        &self,
        limit: usize,
    ) -> Result<Vec<HealthSignal>, Box<dyn std::error::Error + Send + Sync>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut signals = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(signal) = serde_json::from_str::<HealthSignal>(&line) {
                signals.push(signal);
                if signals.len() >= limit {
                    break;
                }
            }
        }
        Ok(signals)
    }

    fn rotate_journal(
        &self,
        max_bytes: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if !self.path.exists() {
            return Ok(false);
        }
        let meta = fs::metadata(&self.path)?;
        if meta.len() > max_bytes {
            let backup = self.path.with_extension("jsonl.old");
            fs::rename(&self.path, backup)?;
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{HealthSignalPayload, SignalSeverity};

    #[test]
    fn appends_and_reads_signals_from_wal() {
        let dir = tempfile::tempdir().expect("tempdir");
        let journal_path = dir.path().join("test_journal.jsonl");
        let journal = LocalObservationJournal::new(&journal_path);

        let signal = HealthSignal::new(
            "node-test",
            SignalSeverity::Warning,
            HealthSignalPayload::OrganSilence {
                organ_id: "test-organ".to_string(),
                silent_duration_seconds: 3600,
            },
            1_786_800_000,
        );

        journal.append_signal(&signal).expect("append signal");

        let read_back = journal.read_signals(10).expect("read signals");
        assert_eq!(read_back.len(), 1);
        assert_eq!(read_back[0].source_node_id, "node-test");
        assert_eq!(read_back[0].signal_id, signal.signal_id);
    }

    #[test]
    fn rotates_journal_when_size_exceeded() {
        let dir = tempfile::tempdir().expect("tempdir");
        let journal_path = dir.path().join("test_journal.jsonl");
        let journal = LocalObservationJournal::new(&journal_path);

        let signal = HealthSignal::new(
            "node-test",
            SignalSeverity::Info,
            HealthSignalPayload::StorageLatency {
                operation: "query".to_string(),
                latency_ms: 50,
                threshold_ms: 100,
            },
            1_786_800_000,
        );

        journal.append_signal(&signal).expect("append signal");

        // Rotate with tiny threshold
        let rotated = journal.rotate_journal(10).expect("rotate");
        assert!(rotated);
        assert!(!journal_path.exists());
        assert!(dir.path().join("test_journal.jsonl.old").exists());
    }
}
