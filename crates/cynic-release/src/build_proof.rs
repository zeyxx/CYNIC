//! Release gate: the builder's verdict on a commit.
//!
//! WHY THIS IS A RELEASE CONCERN, NOT A CLI CONCERN.
//!
//! `contracts/release-gates.json` declares `builder_validation` — "Build
//! validated on builder node" — and the builder is the declared SSOT for build
//! validation. A release that has never been validated there is a release whose
//! evidence does not exist. Until now the proof log was written by the trigger
//! and read only by two display surfaces: `cynic build proofs list` and
//! `cynic build invoice`. Nothing in the promotion path consulted it, so a
//! commit could be promoted with no proof at all and no step would notice.
//!
//! This module is the missing reader. It turns a proof-log reading into a
//! release gate record, and the promotion path now consumes it.
//!
//! FOUR OUTCOMES, THREE OF THEM NOT-PASSING. The distinction is the point:
//!
//!   Converged   — the builder ran and the pipeline passed.            -> pass
//!   Blocked     — the builder ran and the pipeline failed, with reason. -> fail
//!   NeverTested — no proof exists for this commit.                    -> fail
//!   Unreadable  — the log carries lines that are not valid JSON.      -> fail
//!
//! `NeverTested` and `Unreadable` both block, and they are deliberately NOT
//! collapsed into one another: the first says the commit was not validated, the
//! second says the evidence cannot be trusted. An operator fixing one is not
//! fixing the other. Both are absence of proof, and absence is not consent.

use serde::{Deserialize, Serialize};

use crate::types::GateProofRecord;

/// Gate name recorded in `GateProofRecord` for the builder's verdict.
pub const BUILD_PROOF_GATE_NAME: &str = "build_proof";

/// What the proof log says about one commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "observed", rename_all = "snake_case")]
pub enum BuildProofObservation {
    /// No proof exists for this commit: it was never triggered on a runner.
    NeverTested,
    /// The most recent proof says the pipeline converged.
    Converged {
        /// Runner that produced the proof.
        node: String,
    },
    /// The most recent proof says the pipeline blocked.
    Blocked {
        /// Runner that produced the proof.
        node: String,
        /// Why it blocked, in the runner's own words.
        reason: String,
    },
    /// The log carries lines that could not be parsed.
    ///
    /// Not the same as `NeverTested`: evidence was written and is unreadable.
    /// Failing closed here is the whole point — a corrupted log must not read
    /// as a clean record.
    Unreadable {
        /// How many lines were skipped.
        skipped_lines: usize,
    },
    /// A proof exists but its status is one this build does not understand.
    ///
    /// A fourth kind of absence, kept separate for the same reason as the
    /// others: an unrecognised status may mean a newer writer, a schema change,
    /// or a hand-edited line, and each calls for a different response. It is
    /// emphatically not a pass.
    UnknownStatus {
        /// Runner that produced the proof.
        node: String,
        /// The status string as written.
        status: String,
    },
}

impl BuildProofObservation {
    /// Whether this observation permits promoting the commit.
    ///
    /// Only a converged proof does. Everything else is absence of evidence, and
    /// absence is not consent.
    pub fn permits_promotion(&self) -> bool {
        matches!(self, Self::Converged { .. })
    }

    /// Human-readable detail for the gate record.
    pub fn detail(&self) -> String {
        match self {
            Self::Converged { node } => {
                format!("builder proof converged on {node}")
            }
            Self::Blocked { node, reason } => {
                format!("builder proof blocked on {node}: {reason}")
            }
            Self::NeverTested => {
                "no builder proof exists for this commit: it was never validated".to_string()
            }
            Self::Unreadable { skipped_lines } => format!(
                "builder proof log is unreadable: {skipped_lines} line(s) are not valid JSON"
            ),
            Self::UnknownStatus { node, status } => format!(
                "builder proof on {node} carries status '{status}', which this build does not recognise: not a pass"
            ),
        }
    }

    /// Project onto the release gate record shape.
    pub fn to_gate_record(&self, observed_at_unix_s: u64) -> GateProofRecord {
        GateProofRecord {
            name: BUILD_PROOF_GATE_NAME.to_string(),
            passed: self.permits_promotion(),
            detail: self.detail(),
            observed_at_unix_s,
        }
    }
}

/// Reduce proof-log records to the builder's verdict on one commit.
///
/// The MOST RECENT proof for the commit is authoritative: a commit that was
/// blocked and later fixed must be releasable, and one that converged and was
/// then re-triggered into failure must not be. Reading the newest record is
/// what makes a fix actually count.
///
/// Pure: operates on records the caller already read; never touches the log
/// itself. The I/O boundary (default path, read, parse) belongs to the
/// caller's adapter, keeping this decision unit-testable without fixtures.
pub fn reduce_newest_proof(
    proofs: &[serde_json::Value],
    skipped_lines: usize,
) -> BuildProofObservation {
    if proofs.is_empty() {
        // A log with unparseable lines and no matching proof is Unreadable:
        // the absence may be an artefact of the corruption, not a real absence.
        if skipped_lines > 0 {
            return BuildProofObservation::Unreadable { skipped_lines };
        }
        return BuildProofObservation::NeverTested;
    }

    // Newest last, per the reader's contract.
    let newest = proofs.last().expect("non-empty checked above");
    let node = newest
        .get("node")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let status = newest
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    match status {
        "converged" => BuildProofObservation::Converged { node },
        "blocked" => BuildProofObservation::Blocked {
            node,
            reason: newest
                .get("verdict_reason")
                .and_then(|v| v.as_str())
                .unwrap_or("no reason recorded")
                .to_string(),
        },
        // A status this build does not know how to interpret is not a pass.
        other => BuildProofObservation::UnknownStatus {
            node,
            status: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_converged_proof_permits_promotion() {
        assert!(
            BuildProofObservation::Converged {
                node: "cynic-builder-ssh".into()
            }
            .permits_promotion()
        );
        assert!(
            !BuildProofObservation::Blocked {
                node: "cynic-builder-ssh".into(),
                reason: "remote build failed: error[E0432]".into(),
            }
            .permits_promotion()
        );
        assert!(!BuildProofObservation::NeverTested.permits_promotion());
        assert!(!BuildProofObservation::Unreadable { skipped_lines: 1 }.permits_promotion());
        assert!(
            !BuildProofObservation::UnknownStatus {
                node: "cynic-builder-ssh".into(),
                status: "future_status".into(),
            }
            .permits_promotion(),
            "an unrecognised status is not a pass"
        );
    }

    #[test]
    fn never_tested_and_unreadable_are_not_the_same_absence() {
        // Collapsing them would hide a corrupted log behind a clean "not yet
        // validated", and an operator fixing one would not be fixing the other.
        let never = BuildProofObservation::NeverTested.detail();
        let unreadable = BuildProofObservation::Unreadable { skipped_lines: 3 }.detail();
        assert_ne!(never, unreadable);
        assert!(never.contains("never validated"));
        assert!(unreadable.contains("unreadable"));
        assert!(unreadable.contains('3'));
    }

    #[test]
    fn a_blocked_proof_carries_its_reason_into_the_gate_record() {
        let observation = BuildProofObservation::Blocked {
            node: "cynic-builder-ssh".into(),
            reason: "error[E0432]: unresolved import `ServiceProbeSpec`".into(),
        };
        let record = observation.to_gate_record(1_700_000_000);
        assert_eq!(record.name, BUILD_PROOF_GATE_NAME);
        assert!(!record.passed);
        assert!(
            record.detail.contains("E0432"),
            "the operator gets the cause, not just the verdict: {}",
            record.detail
        );
    }

    #[test]
    fn reduce_reads_the_newest_proof_and_ignores_older_ones() {
        let older_blocked = serde_json::json!({ "node": "runner-a", "status": "blocked", "verdict_reason": "broken" });
        let newer_converged = serde_json::json!({ "node": "runner-b", "status": "converged" });
        let observation = reduce_newest_proof(&[older_blocked, newer_converged], 0);
        assert_eq!(
            observation,
            BuildProofObservation::Converged {
                node: "runner-b".into()
            }
        );
        assert!(observation.permits_promotion());
    }

    #[test]
    fn reduce_a_blocked_newest_proof_carries_its_reason() {
        let older_converged = serde_json::json!({ "node": "runner-a", "status": "converged" });
        let newer_blocked = serde_json::json!({ "node": "runner-b", "status": "blocked", "verdict_reason": "error[E0432]" });
        let observation = reduce_newest_proof(&[older_converged, newer_blocked], 0);
        assert_eq!(
            observation,
            BuildProofObservation::Blocked {
                node: "runner-b".into(),
                reason: "error[E0432]".into(),
            }
        );
        assert!(!observation.permits_promotion());
    }

    #[test]
    fn reduce_an_unknown_status_is_not_a_pass() {
        let proof = serde_json::json!({ "node": "runner-a", "status": "some_future_status" });
        let observation = reduce_newest_proof(&[proof], 0);
        assert_eq!(
            observation,
            BuildProofObservation::UnknownStatus {
                node: "runner-a".into(),
                status: "some_future_status".into(),
            }
        );
        assert!(!observation.permits_promotion());
    }

    #[test]
    fn reduce_no_proofs_and_no_skips_is_never_tested() {
        assert_eq!(
            reduce_newest_proof(&[], 0),
            BuildProofObservation::NeverTested
        );
    }

    #[test]
    fn reduce_no_proofs_with_skips_is_unreadable() {
        assert_eq!(
            reduce_newest_proof(&[], 3),
            BuildProofObservation::Unreadable { skipped_lines: 3 }
        );
    }

    #[test]
    fn a_gate_record_round_trips_through_json() {
        let observation = BuildProofObservation::Converged {
            node: "cynic-builder-ssh".into(),
        };
        let json = serde_json::to_string(&observation).expect("serialize");
        let parsed: BuildProofObservation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, observation);
        assert_eq!(parsed.to_gate_record(1).passed, true);
    }

    #[test]
    fn a_missing_proof_blocks_a_release_that_would_otherwise_be_eligible() {
        use crate::contracts::ReleaseGatesContract;
        use crate::evaluator::evaluate_release_candidate;
        use crate::types::ReleaseCandidate;

        // The scenario the gate exists for: everything else is fine, and the
        // commit was never validated on the builder.
        let contract_json = r#"{
            "schema": "cynic.release-gates.v1",
            "status": "normative",
            "gates": {
                "mandatory": [
                    {
                        "name": "build_proof",
                        "description": "builder verdict on the commit",
                        "command": "-",
                        "pass_condition": "converged"
                    }
                ],
                "optional": []
            }
        }"#;
        let contract = ReleaseGatesContract::parse(contract_json).expect("contract parses");
        let candidate = ReleaseCandidate {
            commit_sha: "a".repeat(40),
            branch: "main".into(),
            proposed_version: crate::types::ReleaseVersion::new(0, 9, 0, None),
            commit_messages: vec!["feat: something".into()],
        };

        let proof = BuildProofObservation::NeverTested.to_gate_record(1_700_000_000);
        let verdict = evaluate_release_candidate(&candidate, &contract, &[proof], None);
        assert!(
            !verdict.is_eligible(),
            "a commit with no builder proof must not be releasable"
        );

        let proof = BuildProofObservation::Converged {
            node: "cynic-builder-ssh".into(),
        }
        .to_gate_record(1_700_000_000);
        let verdict = evaluate_release_candidate(&candidate, &contract, &[proof], None);
        assert!(
            verdict.is_eligible(),
            "a converged proof permits the release"
        );
    }
}
