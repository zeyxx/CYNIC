//! ICE evidence classification — the formalized emergence layer.
//!
//! The recurring rustc ICEs on this fleet are not noise to retry away; they are
//! reproducible observations of a substrate fault. This module applies the
//! scientific method in typed form:
//!
//! 1. Observation  — `IceSignalObservation` records only what was measured
//!    (exit code, stderr markers, isolation boundary, replication).
//! 2. Hypothesis   — "the fault is the compiler substrate, not the source" is
//!    only *suspected* after one crash.
//! 3. Replication  — `reproduced_after_rerun` promotes suspicion to
//!    confirmation; an unreproduced crash stays suspect and must not reroute
//!    builds.
//! 4. Falsifier    — a real diagnostic exit (`error[E…]`) classifies as
//!    `SourceJudged`: the toolchain completed its judgement, so the ICE
//!    hypothesis is falsified for that run regardless of stderr content.
//! 5. Attribution  — a confirmed crash is attributed to the substrate node,
//!    never to the source: it feeds `AttemptVerdict::ToolchainFault`, which by
//!    construction cannot carry a source verdict.

use serde::{Deserialize, Serialize};

use super::super::fleet::IsolationKind;

/// One measured rustc failure. Fields are observations only — no conclusion is
/// embedded in the input type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IceSignalObservation {
    pub crate_name: String,
    pub toolchain: String,
    /// `None` = the process died before reporting an exit code (crash signal).
    pub exit_code: Option<i32>,
    pub stderr_markers: Vec<String>,
    pub isolation: IsolationKind,
    /// Replication result: did the identical command crash again on rerun?
    pub reproduced_after_rerun: bool,
}

/// The classification lattice. Ordering encodes evidential strength:
/// SuspectedNotReplicated < CompilerCrashConfirmed, and SourceJudged is
/// orthogonal (it is evidence about the source, not about the substrate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IceClassification {
    /// The toolchain ran to completion and judged the source. Any crash-like
    /// markers in stderr are irrelevant: the judgement falsifies the ICE
    /// hypothesis for this run.
    SourceJudged,
    /// One crash observed, not yet replicated. Hypothesis, not proof.
    SuspectedNotReplicated,
    /// Crash replicated on rerun with no source judgement produced. Confirmed
    /// substrate fault; attribution goes to the node/toolchain pair.
    CompilerCrashConfirmed,
}

/// Pure classifier: observation -> classification.
///
/// Decision order matters and mirrors the falsification hierarchy:
/// 1. A real diagnostic exit code means the compiler judged the source —
///    strongest evidence, checked first so stderr noise cannot override it.
/// 2. A crash (no exit code or explicit ICE/SIGSEGV report) without
///    replication stays *suspected*.
/// 3. Only a replicated crash is confirmed as a compiler crash.
pub fn classify_ice_signal(signal: &IceSignalObservation) -> IceClassification {
    let judged_normally =
        matches!(signal.exit_code, Some(code) if (0..=4).contains(&code) || code == 101);
    if judged_normally
        && !signal
            .stderr_markers
            .iter()
            .any(|marker| is_crash_marker(marker))
    {
        return IceClassification::SourceJudged;
    }
    let crashed_without_judgement = signal.exit_code.is_none()
        || signal
            .stderr_markers
            .iter()
            .any(|marker| is_crash_marker(marker));
    if !crashed_without_judgement {
        return IceClassification::SourceJudged;
    }
    if signal.reproduced_after_rerun {
        IceClassification::CompilerCrashConfirmed
    } else {
        IceClassification::SuspectedNotReplicated
    }
}

fn is_crash_marker(marker: &str) -> bool {
    marker.contains("internal compiler error")
        || marker.contains("SIGSEGV")
        || marker.contains("signal: 11")
        || marker.contains("rustc interrupted")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ice_signal_with_matching_exit_code_is_classified_compiler_crash() {
        let signal = IceSignalObservation {
            crate_name: "cynic-node".to_string(),
            toolchain: "1.96.0".to_string(),
            exit_code: Some(101),
            stderr_markers: vec![
                "internal compiler error".to_string(),
                "signal: 11".to_string(),
            ],
            isolation: IsolationKind::Local,
            reproduced_after_rerun: true,
        };

        assert_eq!(
            classify_ice_signal(&signal),
            IceClassification::CompilerCrashConfirmed
        );
    }

    #[test]
    fn crash_without_exit_code_is_confirmed_even_without_stderr_markers() {
        // SIGSEGV can kill rustc before any diagnostic is flushed; the missing
        // exit code alone is the observation, stderr markers are optional.
        let signal = IceSignalObservation {
            crate_name: "cynic-protocol".to_string(),
            toolchain: "1.96.0".to_string(),
            exit_code: None,
            stderr_markers: Vec::new(),
            isolation: IsolationKind::Local,
            reproduced_after_rerun: true,
        };

        assert_eq!(
            classify_ice_signal(&signal),
            IceClassification::CompilerCrashConfirmed
        );
    }

    #[test]
    fn single_crash_is_suspected_until_reproduced() {
        // Scientific method: one observation forms a hypothesis, replication
        // promotes it. A single unreproduced crash must not reroute builds.
        let signal = IceSignalObservation {
            crate_name: "cynic-mcp".to_string(),
            toolchain: "1.96.0".to_string(),
            exit_code: None,
            stderr_markers: vec!["SIGSEGV".to_string()],
            isolation: IsolationKind::Local,
            reproduced_after_rerun: false,
        };

        assert_eq!(
            classify_ice_signal(&signal),
            IceClassification::SuspectedNotReplicated
        );
    }

    #[test]
    fn real_compile_error_is_source_judged_never_ice() {
        let signal = IceSignalObservation {
            crate_name: "cynic-node".to_string(),
            toolchain: "1.96.0".to_string(),
            exit_code: Some(101),
            stderr_markers: vec!["error[E0599]".to_string()],
            isolation: IsolationKind::Local,
            reproduced_after_rerun: false,
        };

        assert_eq!(
            classify_ice_signal(&signal),
            IceClassification::SourceJudged
        );
    }
}
