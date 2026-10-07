//! Argv execution port — the typed boundary between "what to run" and "running it".
//!
//! This trait and its output type were defined twice, independently:
//! `cynic-lifecycle::ArgvRunner` (L1) and `cynic-vcs::ArgvRunner` (L2), with
//! diverging contracts — `cynic-lifecycle` derived `Serialize`/`Deserialize`
//! because its output is emitted as machine-readable JSON, while `cynic-vcs`
//! derived `PartialEq`/`Eq` and cached its `MapArgvRunner` behind `#[cfg(test)]`,
//! so code in `cynic-vcs` had no way to obtain a mock runner in production.
//!
//! Two definitions of one port means one of two things: an inversion in the DAG
//! (an L2 crate declaring a type an L1 crate already owns) or a silent semantic
//! fork. Both were true. This module is the single home. It is L0 because
//! execution is substrate: the policy that decides *whether* an argv may run
//! already lives here in [`crate::argv_allow`], and the port that runs it must
//! not sit above its own policy.
//!
//! Implementations stay with their adapters — [`crate::argv_allow`] stays pure,
//! `cynic-lifecycle::LocalArgvRunner` stays the production spawner. Only the
//! contract is here.

use serde::{Deserialize, Serialize};

/// Result of one argv execution.
///
/// No process was started when `spawn_error` is set, and `exit_code` is `None`
/// for both that case and a successful spawn that was never reaped. Callers must
/// consult [`ArgvOutput::success`] rather than reading `exit_code` directly, or
/// they will treat a failed spawn as a clean run with no status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgvOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub spawn_error: Option<String>,
}

impl ArgvOutput {
    /// True only when a process ran and reported success.
    pub fn success(&self) -> bool {
        self.spawn_error.is_none() && self.exit_code == Some(0)
    }

    /// One-line rendering for operator surfaces and assertion messages.
    pub fn summary(&self) -> String {
        if let Some(err) = &self.spawn_error {
            return format!("spawn error: {err}");
        }
        let code = self
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "none".into());
        let msg = if !self.stdout.trim().is_empty() {
            self.stdout.trim()
        } else if !self.stderr.trim().is_empty() {
            self.stderr.trim()
        } else {
            "empty output"
        };
        format!("exit {code}: {msg}")
    }

    /// A runner that never started a process, for call sites that must not touch
    /// the host. Keeps test and fail-closed paths from inventing output.
    pub fn not_attempted(reason: &str) -> Self {
        Self {
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            spawn_error: Some(reason.to_string()),
        }
    }
}

/// Argv-only command runner (no shell composition).
///
/// Implementations MUST NOT pass argv through a shell. The argv vector is the
/// trust boundary: composition is what turns a typed argument list into an
/// injection point, and `AGENTS.md` requires typed argv execution for exactly
/// this reason.
pub trait ArgvRunner: Send + Sync {
    fn run(&self, argv: &[&str]) -> ArgvOutput;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_requires_zero_exit_and_no_spawn_error() {
        assert!(ArgvOutput {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            spawn_error: None,
        }
        .success());

        // A failed spawn has no exit code; treating it as success is the bug this
        // documents, because `exit_code == Some(0)` alone would let it pass.
        let spawn_failed = ArgvOutput::not_attempted("exec format error");
        assert!(!spawn_failed.success());
        assert_eq!(spawn_failed.exit_code, None);
        assert!(spawn_failed.summary().starts_with("spawn error:"));
    }

    #[test]
    fn nonzero_exit_is_not_success() {
        assert!(!ArgvOutput {
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "boom".to_string(),
            spawn_error: None,
        }
        .success());
    }

    #[test]
    fn summary_prefers_stdout_then_stderr_then_placeholder() {
        let both = ArgvOutput {
            exit_code: Some(2),
            stdout: "out".into(),
            stderr: "err".into(),
            spawn_error: None,
        };
        assert_eq!(both.summary(), "exit 2: out");

        let stderr_only = ArgvOutput {
            exit_code: Some(2),
            stdout: String::new(),
            stderr: "err".into(),
            spawn_error: None,
        };
        assert_eq!(stderr_only.summary(), "exit 2: err");

        let empty = ArgvOutput {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            spawn_error: None,
        };
        assert_eq!(empty.summary(), "exit 0: empty output");
    }

    #[test]
    fn output_round_trips_as_json() {
        // The derive the `cynic-vcs` copy lacked; it is what makes an
        // ArgvOutput emittable as machine-readable proof.
        let original = ArgvOutput {
            exit_code: Some(0),
            stdout: "ok".into(),
            stderr: String::new(),
            spawn_error: None,
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let parsed: ArgvOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, original);
    }
}
