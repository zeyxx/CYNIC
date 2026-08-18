//! Durable local coordination journal adapter.
//!
//! The journal is a recovery membrane, not a coordination authority. A
//! record is appended only after protocol validation and read-back ordering
//! validation; central coordination remains owned by the configured CoordPort.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use cynic_protocol::{
    COORDINATION_JOURNAL_SCHEMA, CoordinationJournalEntry, validate_journal_entry,
    validate_journal_order,
};

use super::local_spool::LocalEvidenceSpool;

#[derive(Debug, Clone)]
pub struct CoordinationJournal {
    spool: LocalEvidenceSpool,
    append_lock: Arc<std::sync::Mutex<()>>,
}

const JOURNAL_LOCK_RETRY: Duration = Duration::from_millis(5);
const JOURNAL_LOCK_ATTEMPTS: usize = 200;
const JOURNAL_LOCK_STALE_AFTER: Duration = Duration::from_secs(60);

struct ProcessAppendLock {
    path: PathBuf,
}

impl Drop for ProcessAppendLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl CoordinationJournal {
    pub fn from_env() -> Option<Self> {
        let root = std::env::var("CYNIC_COORDINATION_JOURNAL_DIR")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| std::env::var("CYNIC_LOCAL_SPOOL_DIR").ok())?;
        Some(Self::at(
            PathBuf::from(root).join("coordination-journal.jsonl"),
        ))
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            spool: LocalEvidenceSpool::at(path),
            append_lock: Arc::new(std::sync::Mutex::new(())),
        }
    }

    pub fn path(&self) -> &Path {
        self.spool.path()
    }

    pub fn ready(&self) -> bool {
        self.spool.ready()
    }

    pub fn pending(&self) -> Result<usize, String> {
        self.spool.pending()
    }

    pub fn append(&self, entry: &CoordinationJournalEntry) -> Result<(), String> {
        let _guard = self
            .append_lock
            .lock()
            .map_err(|_| "coordination journal lock poisoned".to_string())?;
        let _process_lock = self.acquire_process_append_lock()?;
        self.append_unlocked(entry)
    }

    /// Atomically choose the next sequence and append the generated entry.
    /// This prevents concurrent claims from reusing the same sequence.
    pub fn append_next<F>(&self, build: F) -> Result<u64, String>
    where
        F: FnOnce(u64) -> CoordinationJournalEntry,
    {
        let _guard = self
            .append_lock
            .lock()
            .map_err(|_| "coordination journal lock poisoned".to_string())?;
        let _process_lock = self.acquire_process_append_lock()?;
        let sequence = self
            .spool
            .read::<CoordinationJournalEntry>()?
            .last()
            .map(|entry| entry.sequence.saturating_add(1))
            .unwrap_or(1);
        let entry = build(sequence);
        self.append_unlocked(&entry)?;
        Ok(sequence)
    }

    fn append_unlocked(&self, entry: &CoordinationJournalEntry) -> Result<(), String> {
        let violations = validate_journal_entry(entry);
        if !violations.is_empty() {
            return Err(format!(
                "invalid coordination journal entry: {violations:?}"
            ));
        }
        let mut entries: Vec<CoordinationJournalEntry> = self.spool.read()?;
        entries.push(entry.clone());
        let ordering = validate_journal_order(&entries);
        if !ordering.is_empty() {
            return Err(format!("invalid coordination journal order: {ordering:?}"));
        }
        self.spool.append(entry)
    }

    fn acquire_process_append_lock(&self) -> Result<ProcessAppendLock, String> {
        let lock_path = PathBuf::from(format!("{}.append.lock", self.path().display()));
        if let Some(parent) = lock_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("create coordination journal lock directory: {error}"))?;
        }
        for _ in 0..JOURNAL_LOCK_ATTEMPTS {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(_) => return Ok(ProcessAppendLock { path: lock_path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let stale = std::fs::metadata(&lock_path)
                        .and_then(|metadata| metadata.modified())
                        .ok()
                        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                        .is_some_and(|age| age >= JOURNAL_LOCK_STALE_AFTER);
                    if stale {
                        let _ = std::fs::remove_file(&lock_path);
                    } else {
                        std::thread::sleep(JOURNAL_LOCK_RETRY);
                    }
                }
                Err(error) => {
                    return Err(format!("create coordination journal append lock: {error}"));
                }
            }
        }
        Err("coordination journal append lock remained busy".to_string())
    }

    pub fn read(&self) -> Result<Vec<CoordinationJournalEntry>, String> {
        self.spool.read()
    }

    pub fn drain<F>(&self, consume: F) -> Result<usize, String>
    where
        F: FnMut(CoordinationJournalEntry) -> Result<(), String>,
    {
        self.spool.drain(consume)
    }

    pub fn schema(&self) -> &'static str {
        COORDINATION_JOURNAL_SCHEMA
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::CoordinationJournalPhase;

    fn entry(sequence: u64, fencing_token: u64) -> CoordinationJournalEntry {
        CoordinationJournalEntry {
            schema: COORDINATION_JOURNAL_SCHEMA.to_string(),
            sequence,
            term: 1,
            fencing_token,
            operation: "claim".to_string(),
            target: "target".to_string(),
            phase: CoordinationJournalPhase::Committed,
            proof_ref: Some("sha256:proof".to_string()),
        }
    }

    #[test]
    fn append_rejects_regressed_fencing_before_write() {
        let root = tempfile::tempdir().expect("tempdir");
        let journal = CoordinationJournal::at(root.path().join("journal.jsonl"));
        journal.append(&entry(1, 1)).expect("first entry");
        let error = journal.append(&entry(2, 0)).expect_err("regressed fencing");
        assert!(error.contains("fencing"));
        assert_eq!(journal.pending().expect("pending"), 1);
    }

    #[test]
    fn append_next_assigns_monotonic_sequences() {
        let root = tempfile::tempdir().expect("tempdir");
        let journal = CoordinationJournal::at(root.path().join("journal.jsonl"));
        let first = journal
            .append_next(|sequence| entry(sequence, 1))
            .expect("first");
        let second = journal
            .append_next(|sequence| entry(sequence, 1))
            .expect("second");
        assert_eq!((first, second), (1, 2));
    }

    #[test]
    fn cloned_journals_serialize_append_next_sequences() {
        let root = tempfile::tempdir().expect("tempdir");
        let journal = CoordinationJournal::at(root.path().join("journal.jsonl"));
        let first = journal.clone();
        let second = journal.clone();
        let left = std::thread::spawn(move || {
            first
                .append_next(|sequence| entry(sequence, 1))
                .expect("left append")
        });
        let right = std::thread::spawn(move || {
            second
                .append_next(|sequence| entry(sequence, 1))
                .expect("right append")
        });
        let mut sequences = vec![
            left.join().expect("left join"),
            right.join().expect("right join"),
        ];
        sequences.sort_unstable();
        assert_eq!(sequences, vec![1, 2]);
        assert!(validate_journal_order(&journal.read().expect("journal read")).is_empty());
    }
}
