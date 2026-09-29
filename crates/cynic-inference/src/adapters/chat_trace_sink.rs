//! Append-only JSONL file sink for inference traces.
//!
//! One JSON object per line under an explicit path (default:
//! `$CYNIC_TRACE_LEDGER` or `/var/lib/cynic/evidence/inference_traces.jsonl`).
//! Failures are swallowed by contract — a missing ledger never fails a turn.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::domain::chat_trace::ChatTrace;
use crate::ports::trace_sink_port::TraceSinkPort;

pub const DEFAULT_LEDGER_PATH: &str = "/var/lib/cynic/evidence/inference_traces.jsonl";
pub const LEDGER_ENV_KEY: &str = "CYNIC_TRACE_LEDGER";

/// Resolve the ledger path: explicit > env > principal evidence default.
pub fn resolve_ledger_path(explicit: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit {
        return path.to_path_buf();
    }
    std::env::var(LEDGER_ENV_KEY)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_LEDGER_PATH))
}

/// Thread-safe append sink (file opened per record — low-frequency path).
#[derive(Debug)]
pub struct FileTraceSink {
    path: PathBuf,
    /// Serializes concurrent appends within this process.
    lock: Mutex<()>,
}

impl FileTraceSink {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl TraceSinkPort for FileTraceSink {
    fn record(&self, trace: &ChatTrace) {
        let _guard = self.lock.lock();
        let Ok(line) = serde_json::to_string(trace) else {
            return;
        };
        let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&self.path) else {
            return;
        };
        let _ = writeln!(file, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chat_trace::{TraceStatus, summarize_traces};

    #[test]
    fn appends_one_json_line_per_trace() {
        let dir = tempfile::tempdir().unwrap();
        let sink = FileTraceSink::new(dir.path().join("traces.jsonl"));
        let trace = ChatTrace::new(
            7,
            "sess-1",
            Some("lfm25-2b-q4".into()),
            None,
            TraceStatus::Ok,
            0,
            12,
            34,
            150,
        );
        sink.record(&trace);
        sink.record(&trace);
        let content = std::fs::read_to_string(sink.path()).unwrap();
        assert_eq!(content.lines().count(), 2);
        let back: ChatTrace = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(back.session_id, "sess-1");
        assert_eq!(back.schema, ChatTrace::SCHEMA);
        let summary = summarize_traces(&[back.clone(), back]);
        assert_eq!(summary.turns, 2);
        assert!((summary.ok_rate - 1.0).abs() < 1e-9);
    }

    #[test]
    fn missing_parent_dir_does_not_panic() {
        let sink = FileTraceSink::new("/nonexistent-dir-xyz/traces.jsonl");
        let trace = ChatTrace::new(1, "s", None, None, TraceStatus::Error, 0, 0, 0, 0);
        sink.record(&trace);
    }
}
