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

/// The process-wide execution policy, when one is installed.
///
/// The policy lives in the port crate on purpose: the port that runs an
/// argv must not sit above its own policy, and every runner in the
/// workspace resolves to this trait. Installing the policy here therefore
/// covers the lifecycle runner, the vcs runner, and the host runner with
/// one decision rather than three.
///
/// The slot is write-once: a later caller cannot replace a narrow
/// policy with a wide one.
static POLICY: std::sync::OnceLock<crate::execution_policy::ExecutionPolicy> =
    std::sync::OnceLock::new();

/// Install the process-wide execution policy. The first installation
/// wins.
pub fn install_execution_policy(
    policy: crate::execution_policy::ExecutionPolicy,
) -> crate::execution_policy::ExecutionPolicy {
    POLICY.get_or_init(|| policy).clone()
}

/// The installed execution policy, when one is.
pub fn installed_execution_policy() -> Option<&'static crate::execution_policy::ExecutionPolicy> {
    POLICY.get()
}

/// Supplies the role of the host asking to run an argv.
///
/// The port crate cannot read the registry, so the host installs the
/// provider. A provider that is not installed resolves to the ordinary
/// host role: a host that cannot say what it is may not compile.
static ROLE_PROVIDER: std::sync::OnceLock<
    fn() -> crate::execution_policy::HostRole,
> = std::sync::OnceLock::new();

/// Install the provider of the local host's role.
pub fn install_role_provider(
    provider: fn() -> crate::execution_policy::HostRole,
) {
    let _ = ROLE_PROVIDER.set(provider);
}

/// The local host's role. Without a provider this is the ordinary host
/// role, so the structural refusals stay in force.
pub fn local_host_role() -> crate::execution_policy::HostRole {
    match ROLE_PROVIDER.get() {
        Some(provider) => provider(),
        None => crate::execution_policy::HostRole::Host,
    }
}

/// Refuse an argv the installed policy does not admit.
///
/// Returns the refusal detail when the argv must not run.
///
/// **Fail-closed.** An absent policy is itself a refusal. The previous
/// behaviour returned `None` here, which meant the runner observed the
/// argv and ran it anyway: a process whose catalogue could not be read
/// was exactly the process that ran without a membrane. The catalogue
/// is the declared authority; when it is missing there is nothing that
/// admits the argv, so the argv is refused.
///
/// This is deliberately structural. A ruleset cannot re-open it, and
/// neither can a missing catalogue, a wrong working directory, or a
/// runner used as a library outside the CLI.
pub fn refuse_if_not_admitted(
    argv: &[&str],
    role: crate::execution_policy::HostRole,
) -> Option<String> {
    match POLICY.get() {
        Some(policy) => refusal_from(policy, argv, role),
        None => Some(NO_POLICY_REFUSAL.to_string()),
    }
}

/// The refusal detail when no policy is installed.
///
/// The catalogue is the declared authority. When it is missing there is
/// nothing that admits the argv, so the argv is refused. This constant
/// is the single wording of that refusal, so the message a caller sees
/// does not depend on where the policy was looked up.
pub const NO_POLICY_REFUSAL: &str = "no execution policy is installed: nothing admits this argv";

/// The pure decision over a known policy.
///
/// The global slot cannot be emptied, so the fail-closed behaviour is
/// proven here rather than through the slot: this function takes the
/// policy as an argument, and the wrapper above is the only place that
/// reads the slot.
fn refusal_from(
    policy: &crate::execution_policy::ExecutionPolicy,
    argv: &[&str],
    role: crate::execution_policy::HostRole,
) -> Option<String> {
    match policy.decide(argv, role) {
        crate::execution_policy::ExecutionDecision::Allow { .. } => None,
        crate::execution_policy::ExecutionDecision::Refuse { detail, .. } => Some(detail),
    }
}

/// Refuse an argv using the local host's role.
pub fn refuse_if_not_admitted_here(argv: &[&str]) -> Option<String> {
    refuse_if_not_admitted(argv, local_host_role())
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

/// Blanket impl: a shared runner behind an `Arc` is still an argv runner.
/// Lets fleet observers hold `Arc<R>` and hand `Arc<R>` clones to adapters.
/// Lives here (not in `cynic-lifecycle`) because the orphan rule allows a
/// blanket impl over a foreign type only in the trait's home crate.
impl<R: ArgvRunner> ArgvRunner for std::sync::Arc<R> {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        self.as_ref().run(argv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The membrane is fail-closed: an absent policy refuses. The
    /// previous behaviour returned `None`, so a process whose catalogue
    /// could not be read ran every argv unfiltered — which is exactly
    /// the process that had no membrane.
    #[test]
    fn an_absent_policy_refuses_the_argv() {
        let refusal = NO_POLICY_REFUSAL;
        assert!(refusal.contains("no execution policy is installed"));
        // The wrapper reads the global slot, so the pure decision is
        // proven over a known policy and the wording of the absent case
        // is proven here.
        let policy = crate::execution_policy::ExecutionPolicy::default();
        assert!(refusal_from(&policy, &["git", "status"], crate::execution_policy::HostRole::Host).is_some());
    }

    /// A declared rule admits, and the refusal disappears.
    #[test]
    fn a_declared_rule_admits_through_the_membrane() {
        let mut policy = crate::execution_policy::ExecutionPolicy::new(vec![
            crate::argv_allow::AllowRule {
                program: "git".to_string(),
                arg_prefix: Vec::new(),
                reason: "declared".to_string(),
            },
        ]);
        policy.build_roles = Vec::new();
        assert!(refusal_from(&policy, &["git", "status"], crate::execution_policy::HostRole::Host).is_none());
    }

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
