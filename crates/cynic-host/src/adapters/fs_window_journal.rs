//! FsWindowJournal — JSONL adapter for the window ledger (plan §9 gap 1,
//! layer 2). Append-only writes, tail replay on open; the pure replay
//! logic lives in `cynic_protocol::window_journal` and is never duplicated.
//!
//! Path authority follows the established store chain (state_store pattern):
//! `CYNIC_WINDOW_JOURNAL` env override → `/var/lib/cynic/lifecycle/`
//! → principal-home fallback via passwd lookup → temp dir (last resort,
//! flagged as a residual unknown by callers reading the path).

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use cynic_protocol::window_journal::{
    journal_is_coherent, open_reservations, replay, WindowJournalRecord, WindowJournalState,
};

pub const DEFAULT_WINDOW_JOURNAL_PATH: &str = "/var/lib/cynic/lifecycle/window-journal.jsonl";

#[derive(Debug, thiserror::Error)]
pub enum WindowJournalError {
    #[error("I/O error on {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Append-only window journal backed by one JSONL file.
#[derive(Debug, Clone)]
pub struct FsWindowJournal {
    path: PathBuf,
}

impl FsWindowJournal {
    /// Resolve the journal path: env override → canonical → principal-home
    /// fallback. Mirrors `update_lifecycle::state_store` resolution so all
    /// lifecycle ledgers live under the same authority chain.
    pub fn resolve_path() -> PathBuf {
        if let Ok(override_path) = std::env::var("CYNIC_WINDOW_JOURNAL") {
            return PathBuf::from(override_path);
        }
        let canonical = Path::new(DEFAULT_WINDOW_JOURNAL_PATH);
        if canonical.exists() || canonical.parent().map(|p| p.exists()).unwrap_or(false) {
            return canonical.to_path_buf();
        }
        // Principal-home fallback (same chain as state_store).
        let home = std::env::var("CYNIC_PRINCIPAL_HOME")
            .ok()
            .or_else(|| passwd_home("cynic"))
            .unwrap_or_else(|| "/tmp".to_string());
        PathBuf::from(home)
            .join(".cynic")
            .join("lifecycle")
            .join("window-journal.jsonl")
    }

    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn open_default() -> Self {
        Self::open(Self::resolve_path())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one record with flush + fsync-style durability (flush only;
    /// full fsync is the caller's durability policy decision).
    pub fn append(&self, record: &WindowJournalRecord) -> Result<(), WindowJournalError> {
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|source| WindowJournalError::Io {
                    path: self.path.display().to_string(),
                    source,
                })?;
            }
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|source| WindowJournalError::Io {
                path: self.path.display().to_string(),
                source,
            })?;
        let line = serde_json::to_string(record)?;
        writeln!(file, "{line}").map_err(|source| WindowJournalError::Io {
            path: self.path.display().to_string(),
            source,
        })?;
        file.flush().map_err(|source| WindowJournalError::Io {
            path: self.path.display().to_string(),
            source,
        })
    }

    /// Read back every parseable record. Unparseable lines are skipped but
    /// counted in the returned tuple so corruption stays visible.
    pub fn read_records(&self) -> Result<(Vec<WindowJournalRecord>, usize), WindowJournalError> {
        let mut records = Vec::new();
        let mut corrupt_lines = 0usize;
        if !self.path.exists() {
            return Ok((records, corrupt_lines));
        }
        let file = File::open(&self.path).map_err(|source| WindowJournalError::Io {
            path: self.path.display().to_string(),
            source,
        })?;
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|source| WindowJournalError::Io {
                path: self.path.display().to_string(),
                source,
            })?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            match serde_json::from_str::<WindowJournalRecord>(trimmed) {
                Ok(record) => records.push(record),
                Err(_) => corrupt_lines += 1,
            }
        }
        Ok((records, corrupt_lines))
    }

    /// Replay the whole file into pure state (protocol logic, not duplicated
    /// here). Corrupt lines surface as a violation entry appended to state.
    pub fn replay_state(&self) -> Result<WindowJournalState, WindowJournalError> {
        let (records, corrupt_lines) = self.read_records()?;
        let mut state = replay(&records);
        if corrupt_lines > 0 {
            state
                .violations
                .push(format!("corrupt_journal_lines:{corrupt_lines}"));
        }
        Ok(state)
    }

    /// Convenience for LocalTaskAdmission: reserved budgets of currently
    /// open windows. Empty when the file does not exist yet.
    pub fn open_reservations(
        &self,
    ) -> Result<Vec<cynic_protocol::task_pipeline::EnergyBudget>, WindowJournalError> {
        let state = self.replay_state()?;
        Ok(open_reservations(&state))
    }

    /// Coherence gate: no violations, no dangling opens. A journal failing
    /// this must not back convergence claims (plan gap 1).
    pub fn is_coherent(&self) -> Result<bool, WindowJournalError> {
        Ok(journal_is_coherent(&self.replay_state()?))
    }
}

/// Minimal /etc/passwd lookup for the cynic principal home (no libc dep):
/// matches the established fallback used by update_lifecycle::state_store.
fn passwd_home(user: &str) -> Option<String> {
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    passwd.lines().find_map(|line| {
        let mut fields = line.split(':');
        let name = fields.next()?;
        if name != user {
            return None;
        }
        fields.nth(4).map(|home| home.trim().to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::task_pipeline::{EnergyBudget, EnergyConsumed};
    use cynic_protocol::window_journal::{
        WindowClose, WindowOpen, WindowStatus, WINDOW_JOURNAL_CLOSE_SCHEMA,
        WINDOW_JOURNAL_OPEN_SCHEMA,
    };
    use tempfile::NamedTempFile;

    fn budget(wall_ms: u64, tokens: u64) -> EnergyBudget {
        EnergyBudget {
            wall_time_ms: wall_ms,
            inference_tokens: Some(tokens),
            cpu_ms: None,
            ram_bytes_s: None,
        }
    }

    fn open(id: &str, tokens: u64) -> WindowJournalRecord {
        WindowJournalRecord::Open(WindowOpen {
            schema: WINDOW_JOURNAL_OPEN_SCHEMA.to_string(),
            window_id: id.into(),
            host_id: "test-host".into(),
            parent_window_id: None,
            epoch: 1,
            budget: budget(1_000, tokens),
            capacity_ref: "cap-1".into(),
            opened_at_unix_ms: 42,
        })
    }

    fn close(id: &str) -> WindowJournalRecord {
        WindowJournalRecord::Close(WindowClose {
            schema: WINDOW_JOURNAL_CLOSE_SCHEMA.to_string(),
            window_id: id.into(),
            status: WindowStatus::Completed,
            consumed: EnergyConsumed {
                wall_time_ms: 800,
                inference_tokens: None,
                cpu_ms: None,
                ram_bytes_s: None,
            },
            evidence_refs: vec![],
        })
    }

    #[test]
    fn append_then_replay_roundtrips() {
        let temp = NamedTempFile::new().unwrap();
        let journal = FsWindowJournal::open(temp.path());
        journal.append(&open("w1", 10)).unwrap();
        journal.append(&close("w1")).unwrap();

        let state = journal.replay_state().unwrap();
        assert!(journal_is_coherent(&state));
        assert_eq!(state.closed.len(), 1);
        assert!(state.open.is_empty());
    }

    #[test]
    fn open_reservations_reflects_dangling_opens() {
        let temp = NamedTempFile::new().unwrap();
        let journal = FsWindowJournal::open(temp.path());
        journal.append(&open("w1", 30)).unwrap();
        journal.append(&open("w2", 20)).unwrap();

        let reservations = journal.open_reservations().unwrap();
        assert_eq!(reservations.len(), 2);
        let total: u64 = reservations.iter().filter_map(|b| b.inference_tokens).sum();
        assert_eq!(total, 50);
    }

    #[test]
    fn missing_file_is_an_empty_coherent_journal() {
        let journal = FsWindowJournal::open("/tmp/cynic-nonexistent-journal-test.jsonl");
        let _ = std::fs::remove_file(journal.path());
        assert!(journal.open_reservations().unwrap().is_empty());
        assert!(journal.is_coherent().unwrap());
    }

    #[test]
    fn corrupt_line_surfaces_as_violation() {
        let temp = NamedTempFile::new().unwrap();
        {
            let mut f = File::options().append(true).open(temp.path()).unwrap();
            writeln!(f, "{{not json at all").unwrap();
        }
        let journal = FsWindowJournal::open(temp.path());
        journal.append(&open("w1", 5)).unwrap();

        let state = journal.replay_state().unwrap();
        assert!(state
            .violations
            .iter()
            .any(|v| v.starts_with("corrupt_journal_lines:")));
    }

    #[test]
    fn resolve_path_prefers_env_override() {
        let guard = override_env("/tmp/env-override-journal.jsonl");
        let resolved = FsWindowJournal::resolve_path();
        restore_env(guard);
        assert_eq!(
            resolved,
            PathBuf::from("/tmp/env-override-journal.jsonl"),
            "env override wins over canonical path"
        );
    }

    // Serial access to the process-wide env var across parallel tests.
    // set_var/remove_var are unsafe in edition 2024; the mutex guarantees
    // the unsafe window is exclusive, matching runtime_layout's EnvOverride.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    const ENV_KEY: &str = "CYNIC_WINDOW_JOURNAL";

    fn override_env(value: &str) -> Option<String> {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let previous = std::env::var(ENV_KEY).ok();
        // SAFETY: single-threaded within the ENV_LOCK critical section.
        #[allow(unsafe_op_in_unsafe_fn)]
        unsafe {
            std::env::set_var(ENV_KEY, value)
        };
        previous
    }

    fn restore_env(previous: Option<String>) {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: same exclusive critical section as override_env.
        unsafe {
            match previous {
                Some(v) => std::env::set_var(ENV_KEY, v),
                None => std::env::remove_var(ENV_KEY),
            }
        }
    }
}
