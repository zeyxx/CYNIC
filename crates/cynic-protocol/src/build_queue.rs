//! Build queue ordering: FIFO by commit time with SHA deduplication.
//!
//! Pure policy for the scheduler loop. It operates on a lightweight candidate
//! view so callers map their record shape once and tests fabricate candidates
//! in one line each. Terminal records never queue; later duplicates of an
//! idempotency key keep only the earliest commit. Fairness across branches is
//! explicitly out of scope until branch identity is recorded at admission.

use serde::{Deserialize, Serialize};

pub const BUILD_QUEUE_PLAN_SCHEMA: &str = "cynic.build_queue_plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedCandidate {
    pub workload_id: String,
    pub idempotency_key: String,
    pub committed_at_ms: u64,
    /// True for terminal records and non-build workloads: never queued,
    /// counted separately so silence is impossible.
    pub terminal: bool,
    pub is_build: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildQueuePlan {
    pub schema: String,
    pub ordered: Vec<String>,
    pub terminal_skipped: usize,
    pub non_build_skipped: usize,
    pub duplicates_skipped: usize,
}

/// Order candidates FIFO by commit time, deduplicated by idempotency key.
pub fn plan_build_queue(candidates: Vec<QueuedCandidate>) -> BuildQueuePlan {
    let mut terminal_skipped = 0;
    let mut non_build_skipped = 0;
    let mut duplicates_skipped = 0;
    let mut eligible: Vec<QueuedCandidate> = Vec::new();
    for candidate in candidates {
        if candidate.terminal {
            terminal_skipped += 1;
        } else if !candidate.is_build {
            non_build_skipped += 1;
        } else {
            eligible.push(candidate);
        }
    }
    eligible.sort_by(|left, right| {
        left.committed_at_ms
            .cmp(&right.committed_at_ms)
            .then_with(|| left.workload_id.cmp(&right.workload_id))
    });
    let mut seen = std::collections::BTreeSet::new();
    let mut ordered = Vec::new();
    for candidate in eligible {
        if seen.insert(candidate.idempotency_key.clone()) {
            ordered.push(candidate.workload_id);
        } else {
            duplicates_skipped += 1;
        }
    }
    BuildQueuePlan {
        schema: BUILD_QUEUE_PLAN_SCHEMA.to_string(),
        ordered,
        terminal_skipped,
        non_build_skipped,
        duplicates_skipped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, key: &str, at_ms: u64) -> QueuedCandidate {
        QueuedCandidate {
            workload_id: id.to_string(),
            idempotency_key: key.to_string(),
            committed_at_ms: at_ms,
            terminal: false,
            is_build: true,
        }
    }

    #[test]
    fn orders_fifo_and_dedupes_by_idempotency_key() {
        let plan = plan_build_queue(vec![
            candidate("w-new", "key:b", 200),
            candidate("w-old", "key:a", 100),
            candidate("w-dup", "key:a", 300),
        ]);

        assert_eq!(plan.ordered, vec!["w-old".to_string(), "w-new".to_string()]);
        assert_eq!(plan.duplicates_skipped, 1);
    }

    #[test]
    fn counts_terminal_and_non_build_without_queueing() {
        let mut done = candidate("w-done", "key:c", 50);
        done.terminal = true;
        let mut other = candidate("w-other", "key:d", 60);
        other.is_build = false;
        let plan = plan_build_queue(vec![done, other, candidate("w-ok", "key:e", 70)]);

        assert_eq!(plan.ordered, vec!["w-ok".to_string()]);
        assert_eq!(plan.terminal_skipped, 1);
        assert_eq!(plan.non_build_skipped, 1);
    }

    #[test]
    fn empty_input_stays_empty_not_failed() {
        let plan = plan_build_queue(Vec::new());

        assert!(plan.ordered.is_empty());
        assert_eq!(plan.duplicates_skipped, 0);
    }
}
