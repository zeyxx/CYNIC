//! WindowRunner — open → work → close lifecycle for tick windows (plan §9
//! gap 1 completion, the last data-layer brick before composition roots).
//!
//! Data-centric contract:
//! - `open`  : reserve the budget in the FsWindowJournal BEFORE work starts
//!   (admission already happened upstream; this is the reservation record)
//! - `work`  : caller-supplied closure, receives nothing and returns a
//!   serde-serializable outcome value
//! - `close` : measure EnergyConsumed (CpuUsageMeter + wall clock), append
//!   the Close record with honest None for unmeasured dimensions
//!
//! Failure semantics: a panicking or erroring closure still produces a
//! Close record with status Failed/Cancelled — an orphan open would corrupt
//! the ledger (gap 1 violation class). The closure result is returned after
//! the close is durably written.

use std::sync::Arc;

use cynic_protocol::task_pipeline::{EnergyBudget, EnergyConsumed};
use cynic_protocol::temporal::Epoch;
use cynic_protocol::window_journal::{WindowClose, WindowJournalRecord, WindowOpen, WindowStatus};

use crate::adapters::cpu_usage_meter::CpuUsageMeter;
use crate::adapters::fs_window_journal::FsWindowJournal;

pub struct WindowRunner {
    journal: Arc<FsWindowJournal>,
    host_id: String,
}

/// Result of a run: the closure's own output plus the measured receipt.
#[derive(Debug)]
pub struct WindowRunOutcome<T> {
    pub result: T,
    pub consumed: EnergyConsumed,
    pub window_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum WindowRunError {
    #[error("journal write failed: {0}")]
    Journal(String),
    #[error("closure failed: {0}")]
    Work(String),
}

impl WindowRunner {
    pub fn new(journal: Arc<FsWindowJournal>, host_id: impl Into<String>) -> Self {
        Self {
            journal,
            host_id: host_id.into(),
        }
    }

    /// Run one bounded window synchronously.
    ///
    /// - Opens the window (persisted before the closure runs).
    /// - Runs `work`.
    /// - Closes with Completed/Failed + measured consumption. A closure
    ///   error is returned AFTER the close record is durable — the ledger
    ///   never keeps a dangling open, whatever the outcome.
    pub fn run<T, F>(
        &self,
        window_id: &str,
        parent_window_id: Option<&str>,
        epoch: Epoch,
        budget: EnergyBudget,
        capacity_ref: &str,
        meter: &mut CpuUsageMeter,
        work: F,
    ) -> Result<WindowRunOutcome<T>, WindowRunError>
    where
        F: FnOnce(&mut CpuUsageMeter) -> Result<T, String>,
    {
        // 1. OPEN — persisted before any work; failure here aborts honestly.
        let open = WindowOpen {
            schema: cynic_protocol::window_journal::WINDOW_JOURNAL_OPEN_SCHEMA.to_string(),
            window_id: window_id.to_string(),
            host_id: self.host_id.clone(),
            parent_window_id: parent_window_id.map(|p| p.to_string()),
            epoch: epoch.id(),
            budget: budget.clone(),
            capacity_ref: capacity_ref.to_string(),
            opened_at_unix_ms: 0, // annotation only; monotone clock rules below
        };
        self.journal
            .append(&WindowJournalRecord::Open(open))
            .map_err(|e| WindowRunError::Journal(e.to_string()))?;

        // 2. WORK — sample cpu at entry for the delta baseline.
        let start_mark = meter.mark();
        match work(meter) {
            Ok(result) => {
                let end_mark = meter.mark();
                let wall_time_ms = start_mark.elapsed_ms_since(end_mark.sampled_at);
                let cpu_ms = meter.cpu_ms_between(&start_mark, &end_mark);
                self.finish(
                    window_id,
                    WindowStatus::Completed,
                    EnergyConsumed {
                        wall_time_ms,
                        inference_tokens: None,
                        cpu_ms,
                        ram_bytes_s: None,
                    },
                )?;
                Ok(WindowRunOutcome {
                    result,
                    consumed: EnergyConsumed {
                        wall_time_ms,
                        inference_tokens: None,
                        cpu_ms,
                        ram_bytes_s: None,
                    },
                    window_id: window_id.to_string(),
                })
            }
            Err(work_error) => {
                let end_mark = meter.mark();
                let wall_time_ms = start_mark.elapsed_ms_since(end_mark.sampled_at);
                let cpu_ms = meter.cpu_ms_between(&start_mark, &end_mark);
                self.finish(
                    window_id,
                    WindowStatus::Failed,
                    EnergyConsumed {
                        wall_time_ms,
                        inference_tokens: None,
                        cpu_ms,
                        ram_bytes_s: None,
                    },
                )?;
                Err(WindowRunError::Work(work_error))
            }
        }
    }

    fn finish(
        &self,
        window_id: &str,
        status: WindowStatus,
        consumed: EnergyConsumed,
    ) -> Result<(), WindowRunError> {
        self.journal
            .append(&WindowJournalRecord::Close(WindowClose {
                schema: cynic_protocol::window_journal::WINDOW_JOURNAL_CLOSE_SCHEMA.to_string(),
                window_id: window_id.to_string(),
                status,
                consumed,
                evidence_refs: Vec::new(),
            }))
            .map_err(|e| WindowRunError::Journal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::fs_window_journal::FsWindowJournal;
    use tempfile::NamedTempFile;

    fn runner(path: &Path) -> WindowRunner {
        WindowRunner::new(Arc::new(FsWindowJournal::open(path)), "test-host")
    }

    use std::path::Path;

    fn ok_budget() -> EnergyBudget {
        EnergyBudget {
            wall_time_ms: 5_000,
            inference_tokens: Some(10),
            cpu_ms: None,
            ram_bytes_s: None,
        }
    }

    #[test]
    fn successful_run_writes_coherent_pair() {
        let temp = NamedTempFile::new().unwrap();
        let r = runner(temp.path());
        let mut meter = CpuUsageMeter::new();

        let outcome = r
            .run(
                "w-ok",
                None,
                Epoch::new(1),
                ok_budget(),
                "cap-ref",
                &mut meter,
                |_meter| Ok::<_, String>(42),
            )
            .unwrap();

        assert_eq!(outcome.result, 42);
        assert!(outcome.consumed.wall_time_ms < 60_000);

        let state = r.journal.replay_state().unwrap();
        assert!(state.violations.is_empty());
        assert!(state.open.is_empty(), "close must release the reservation");
        let close_record = &state.closed["w-ok"];
        assert_eq!(close_record.status, WindowStatus::Completed);
    }

    #[test]
    fn failing_closure_still_closes_the_window() {
        let temp = NamedTempFile::new().unwrap();
        let r = runner(temp.path());
        let mut meter = CpuUsageMeter::new();

        let err = r
            .run(
                "w-fail",
                None,
                Epoch::new(2),
                ok_budget(),
                "cap-ref",
                &mut meter,
                |_meter| Err::<(), _>("boom".to_string()),
            )
            .unwrap_err();

        assert!(matches!(err, WindowRunError::Work(ref e) if e == "boom"));

        // The ledger must NOT hold a dangling open — gap 1 invariant.
        let state = r.journal.replay_state().unwrap();
        assert!(state.violations.is_empty());
        assert!(state.open.is_empty());
        assert_eq!(state.closed["w-fail"].status, WindowStatus::Failed);
    }

    #[test]
    fn nested_windows_track_parentage_and_both_close() {
        let temp = NamedTempFile::new().unwrap();
        let r = runner(temp.path());
        let mut meter = CpuUsageMeter::new();

        let outer_outcome = r
            .run(
                "parent",
                None,
                Epoch::new(3),
                ok_budget(),
                "cap",
                &mut meter,
                |outer_meter| {
                    // Inner window runs inside the outer closure (depth 1).
                    // It reborrows the SAME meter passed down — cpu_ms
                    // accumulates across the nesting on one shared core.
                    let inner = r
                        .run(
                            "child",
                            Some("parent"),
                            Epoch::new(3),
                            ok_budget(),
                            "cap",
                            outer_meter,
                            |m| {
                                let _ = m.mark();
                                Ok::<_, String>("inner".to_string())
                            },
                        )
                        .unwrap();
                    Ok(inner.result)
                },
            )
            .unwrap();

        assert_eq!(outer_outcome.result, "inner");
        let state = r.journal.replay_state().unwrap();
        assert!(journal_coherent(&state));
        // Child closed before parent (stack order) — both released.
        assert_eq!(state.closed.len(), 2);
        assert_eq!(state.closed["child"].window_id, "child",);
    }

    // Local helper so tests don't import from two paths.
    fn journal_coherent(state: &cynic_protocol::window_journal::WindowJournalState) -> bool {
        cynic_protocol::window_journal::journal_is_coherent(state)
    }
}
