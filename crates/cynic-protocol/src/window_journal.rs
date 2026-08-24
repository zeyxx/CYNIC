//! Window journal — JSONL append-only ledger for tick windows (plan §9 gap 1).
//!
//! Layer order (bottom-up, data-centric):
//!   1. THIS FILE  — pure record types + journal validation (protocol)
//!   2. next       — FsWindowJournal adapter (append/open/close/replay)
//!   3. then       — LocalTaskAdmission consumes replayed open reservations
//!
//! Ledger semantics (gap 4): reserving = the `open` record's budget field;
//! debiting = the `close` record's consumed fields. The pair matched by
//! window_id is the boundary proof (open without close = still reserved;
//! close without open = corrupt journal → explicit error, never ignored).
//!
//! Compression path (documented contract, not yet implemented): once a
//! window's digest is retained via a PoH batch or digest index, the record
//! body becomes compactable — keep digests, drop bodies.

use serde::{Deserialize, Serialize};

pub const WINDOW_JOURNAL_OPEN_SCHEMA: &str = "cynic.window_journal.open.v1";
pub const WINDOW_JOURNAL_CLOSE_SCHEMA: &str = "cynic.window_journal.close.v1";

/// One end of a window lifecycle. `kind` discriminates open/close so a
/// single append-only file holds both and replay reconstructs state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WindowJournalRecord {
    Open(WindowOpen),
    Close(WindowClose),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowOpen {
    pub schema: String,
    pub window_id: String,
    pub host_id: String,
    /// Assurance schéma pour le nesting (gap 3). None = fenêtre racine.
    #[serde(default)]
    pub parent_window_id: Option<String>,
    /// Logical epoch local to the host — never a physical clock.
    pub epoch: u64,
    /// Reserved budget (the reservation itself, gap 4).
    pub budget: crate::task_pipeline::EnergyBudget,
    /// Reference to the CapacityObservation admitted against.
    pub capacity_ref: String,
    /// Non-authoritative annotation (unix ms). Durées/fraîcheur vivent
    /// côté horloge monotone de l'adaptateur, jamais ici.
    pub opened_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowClose {
    pub schema: String,
    pub window_id: String,
    /// Outcome of the bounded action inside the window.
    pub status: WindowStatus,
    /// Actually consumed energy measured by the adapter (None = unmeasured
    /// dimension, unknown du receipt — jamais zéro).
    pub consumed: crate::task_pipeline::EnergyConsumed,
    /// Read-back proof refs produced by the work done in this window.
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowStatus {
    Completed,
    Failed,
    Cancelled,
}

impl WindowJournalRecord {
    pub fn window_id(&self) -> &str {
        match self {
            Self::Open(o) => &o.window_id,
            Self::Close(c) => &c.window_id,
        }
    }
}

// ---------------------------------------------------------------------------
// Pure replay — records in, journal state out. No I/O here.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WindowJournalState {
    /// Currently open windows by id (reserved budgets, gap 4 ledger head).
    pub open: std::collections::BTreeMap<String, WindowOpen>,
    /// Completed boundaries by id (kept for read-back; compaction target).
    pub closed: std::collections::BTreeMap<String, WindowClose>,
    /// Corruption found during replay — surfaced, never silently dropped.
    pub violations: Vec<String>,
}

/// Replay an ordered slice of records into journal state.
///
/// Violations (each also recorded in `violations`):
/// - duplicate open for the same id (second open ignored, first kept);
/// - close without matching open (recorded as orphan close, not applied);
/// - duplicate close for an already-closed id (second ignored).
pub fn replay(records: &[WindowJournalRecord]) -> WindowJournalState {
    let mut state = WindowJournalState::default();
    for record in records {
        match record {
            WindowJournalRecord::Open(open) => {
                if state.open.contains_key(&open.window_id) {
                    state
                        .violations
                        .push(format!("duplicate_open:{}", open.window_id));
                    continue;
                }
                if state.closed.contains_key(&open.window_id) {
                    state
                        .violations
                        .push(format!("reopen_after_close:{}", open.window_id));
                    continue;
                }
                state.open.insert(open.window_id.clone(), open.clone());
            }
            WindowJournalRecord::Close(close) => {
                if let Some(_open) = state.open.remove(&close.window_id) {
                    if state.closed.contains_key(&close.window_id) {
                        // unreachable: remove above guarantees absence; kept
                        // for symmetry with the violation taxonomy.
                        continue;
                    }
                    state
                        .closed
                        .insert(close.window_id.clone(), close.clone());
                } else if state.closed.contains_key(&close.window_id) {
                    state
                        .violations
                        .push(format!("duplicate_close:{}", close.window_id));
                } else {
                    state
                        .violations
                        .push(format!("orphan_close:{}", close.window_id));
                }
            }
        }
    }
    state
}

/// Reserved budgets of currently-open windows — exactly what
/// `admit_task`/`LocalTaskAdmission::open_reservations` consume.
pub fn open_reservations(state: &WindowJournalState) -> Vec<crate::task_pipeline::EnergyBudget> {
    state
        .open
        .values()
        .map(|open| open.budget.clone())
        .collect()
}

/// Boundary proof check: every open has exactly one close and vice versa,
/// with no violations. A journal that fails this cannot back convergence
/// claims (plan gap 1).
pub fn journal_is_coherent(state: &WindowJournalState) -> bool {
    state.violations.is_empty() && state.open.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_pipeline::{EnergyBudget, EnergyConsumed};

    fn budget(wall_ms: u64, tokens: Option<u64>) -> EnergyBudget {
        EnergyBudget {
            wall_time_ms: wall_ms,
            inference_tokens: tokens,
            cpu_ms: None,
            ram_bytes_s: None,
        }
    }

    fn open(id: &str, tokens: u64) -> WindowJournalRecord {
        WindowJournalRecord::Open(WindowOpen {
            schema: WINDOW_JOURNAL_OPEN_SCHEMA.to_string(),
            window_id: id.to_string(),
            host_id: "h".into(),
            parent_window_id: None,
            epoch: crate::temporal::Epoch::new(7).id(),
            budget: budget(1_000, Some(tokens)),
            capacity_ref: "cap-ref-1".into(),
            opened_at_unix_ms: 0,
        })
    }

    fn close(id: &str, tokens: u64) -> WindowJournalRecord {
        WindowJournalRecord::Close(WindowClose {
            schema: WINDOW_JOURNAL_CLOSE_SCHEMA.to_string(),
            window_id: id.to_string(),
            status: WindowStatus::Completed,
            consumed: EnergyConsumed {
                wall_time_ms: 900,
                inference_tokens: Some(tokens),
                cpu_ms: None,
                ram_bytes_s: None,
            },
            evidence_refs: vec!["sha256:proof".into()],
        })
    }

    #[test]
    fn open_then_close_leaves_empty_open_set() {
        let state = replay(&[open("w1", 10), close("w1", 8)]);
        assert!(state.open.is_empty());
        assert_eq!(state.closed.len(), 1);
        assert!(state.violations.is_empty());
        assert!(journal_is_coherent(&state));
        assert!(open_reservations(&state).is_empty());
    }

    #[test]
    fn open_without_close_stays_reserved() {
        let state = replay(&[open("w1", 10), open("w2", 5)]);
        assert_eq!(state.open.len(), 2);
        let reservations = open_reservations(&state);
        assert_eq!(reservations.len(), 2);
        // Both reservations carry their requested token budgets — this vec
        // is what admit_task sums against the envelope.
        let total: u64 = reservations
            .iter()
            .filter_map(|b| b.inference_tokens)
            .sum();
        assert_eq!(total, 15);
    }

    #[test]
    fn orphan_close_is_a_violation_not_silence() {
        let state = replay(&[close("ghost", 1)]);
        assert_eq!(state.closed.len(), 0);
        assert!(state
            .violations
            .iter()
            .any(|v| v == "orphan_close:ghost"));
        assert!(!journal_is_coherent(&state));
    }

    #[test]
    fn duplicate_open_keeps_first_and_records_violation() {
        let state = replay(&[open("w1", 10), open("w1", 999)]);
        assert_eq!(state.open["w1"].budget.inference_tokens, Some(10));
        assert!(state.violations.iter().any(|v| v == "duplicate_open:w1"));
    }

    #[test]
    fn duplicate_close_is_recorded() {
        let state = replay(&[open("w1", 10), close("w1", 5), close("w1", 6)]);
        assert_eq!(state.closed["w1"].consumed.inference_tokens, Some(5));
        assert!(state
            .violations
            .iter()
            .any(|v| v == "duplicate_close:w1"));
    }

    #[test]
    fn roundtrip_through_jsonl_shape() {
        // Serialize both ends like the adapter will write them, then parse
        // back and replay — proves the serde tags survive disk format.
        let recs = vec![open("w1", 10), close("w1", 8)];
        let lines: Vec<String> = recs
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect();
        let parsed: Vec<WindowJournalRecord> = lines
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        let state = replay(&parsed);
        assert!(journal_is_coherent(&state));
        assert_eq!(state.closed["w1"].consumed.wall_time_ms, 900);
    }
}
