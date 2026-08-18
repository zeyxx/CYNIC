//! Content-addressable coordination journal contract.
//!
//! The protocol is backend-neutral. A durable adapter may persist these
//! entries locally or replicate them, but a caller must not treat an entry as
//! committed unless the backend read-back validates its sequence, term,
//! fencing token, and proof reference.

use serde::{Deserialize, Serialize};

pub const COORDINATION_JOURNAL_SCHEMA: &str = "cynic.coordination.journal_entry.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationJournalPhase {
    Prepared,
    Committed,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinationJournalEntry {
    pub schema: String,
    pub sequence: u64,
    pub term: u64,
    pub fencing_token: u64,
    pub operation: String,
    pub target: String,
    pub phase: CoordinationJournalPhase,
    pub proof_ref: Option<String>,
}

pub fn validate_journal_entry(entry: &CoordinationJournalEntry) -> Vec<&'static str> {
    let mut violations = Vec::new();
    if entry.schema != COORDINATION_JOURNAL_SCHEMA {
        violations.push("coordination_journal_schema_invalid");
    }
    if entry.sequence == 0 {
        violations.push("coordination_journal_sequence_missing");
    }
    if entry.term == 0 {
        violations.push("coordination_journal_term_missing");
    }
    if entry.fencing_token == 0 {
        violations.push("coordination_journal_fencing_missing");
    }
    if entry.operation.trim().is_empty() {
        violations.push("coordination_journal_operation_missing");
    }
    if entry.target.trim().is_empty() {
        violations.push("coordination_journal_target_missing");
    }
    if entry.phase == CoordinationJournalPhase::Committed && entry.proof_ref.is_none() {
        violations.push("coordination_journal_commit_proof_missing");
    }
    violations
}

pub fn validate_journal_order(entries: &[CoordinationJournalEntry]) -> Vec<&'static str> {
    let mut violations = Vec::new();
    let mut previous_sequence = 0;
    let mut previous_term = 0;
    let mut previous_fencing = 0;
    for entry in entries {
        violations.extend(validate_journal_entry(entry));
        if entry.sequence <= previous_sequence {
            violations.push("coordination_journal_sequence_not_monotonic");
        }
        if entry.term < previous_term {
            violations.push("coordination_journal_term_regressed");
        }
        if entry.fencing_token < previous_fencing {
            violations.push("coordination_journal_fencing_regressed");
        }
        previous_sequence = entry.sequence;
        previous_term = entry.term;
        previous_fencing = entry.fencing_token;
    }
    violations.sort_unstable();
    violations.dedup();
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        sequence: u64,
        fencing_token: u64,
        phase: CoordinationJournalPhase,
    ) -> CoordinationJournalEntry {
        CoordinationJournalEntry {
            schema: COORDINATION_JOURNAL_SCHEMA.to_string(),
            sequence,
            term: 2,
            fencing_token,
            operation: "claim".to_string(),
            target: "crates/cynic-protocol".to_string(),
            phase,
            proof_ref: (phase == CoordinationJournalPhase::Committed)
                .then(|| "sha256:proof".to_string()),
        }
    }

    #[test]
    fn committed_entry_requires_read_back_proof() {
        let mut committed = entry(1, 1, CoordinationJournalPhase::Committed);
        committed.proof_ref = None;
        assert!(
            validate_journal_entry(&committed)
                .contains(&"coordination_journal_commit_proof_missing")
        );
    }

    #[test]
    fn journal_order_rejects_stale_fencing() {
        let entries = vec![
            entry(1, 1, CoordinationJournalPhase::Prepared),
            entry(2, 0, CoordinationJournalPhase::Aborted),
        ];
        assert!(
            validate_journal_order(&entries).contains(&"coordination_journal_fencing_regressed")
        );
    }
}
