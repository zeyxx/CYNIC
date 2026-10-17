//! Empirical argv allowlist — pure verdict over observed argv (no I/O).
//!
//! Observe mode: every argv executed through `ArgvRunner` receives a
//! verdict. Against an empty ruleset everything is `WouldDeny` but still
//! executes — the observation log is the empirical source from which
//! admission rules are promoted. Deny-by-default is enforced by a later
//! slice; this module only names the verdict.

use serde::{Deserialize, Serialize};

/// Schema of one observation line emitted by the observing runner.
pub const ARGV_OBSERVATION_SCHEMA: &str = "cynic.argv_observation.v1";

/// One admission rule: a program plus an argument prefix.
/// An empty `arg_prefix` admits any invocation of the program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowRule {
    pub program: String,
    pub arg_prefix: Vec<String>,
    /// Why this rule exists (observation window or admission ref).
    pub reason: String,
}

/// Pure verdict for one argv against a ruleset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArgvVerdict {
    Admitted { rule: String },
    WouldDeny { reason: String },
}

impl ArgvVerdict {
    pub fn admitted(&self) -> bool {
        matches!(self, ArgvVerdict::Admitted { .. })
    }

    pub fn label(&self) -> &'static str {
        match self {
            ArgvVerdict::Admitted { .. } => "admitted",
            ArgvVerdict::WouldDeny { .. } => "would_deny",
        }
    }
}

/// Pure decision: first rule whose program and arg prefix match wins.
/// Empty argv and rule-less programs are `WouldDeny`, never errors —
/// the caller decides whether observe mode still executes.
pub fn observe_argv_verdict(argv: &[&str], rules: &[AllowRule]) -> ArgvVerdict {
    let program = argv.first().copied().unwrap_or("");
    if program.is_empty() {
        return ArgvVerdict::WouldDeny {
            reason: "empty argv".to_string(),
        };
    }
    for rule in rules {
        if rule.program != program {
            continue;
        }
        if argv.len() - 1 < rule.arg_prefix.len() {
            continue;
        }
        let prefix_matches = rule
            .arg_prefix
            .iter()
            .enumerate()
            .all(|(i, expected)| argv[i + 1] == expected);
        if prefix_matches {
            return ArgvVerdict::Admitted {
                rule: rule.reason.clone(),
            };
        }
    }
    ArgvVerdict::WouldDeny {
        reason: format!("no rule admits program {program:?}"),
    }
}

/// One observation line. Timestamping is the logger's job (I/O);
/// this shape stays pure so verdict and persistence never couple.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgvObservation {
    pub schema: String,
    pub argv: Vec<String>,
    pub verdict: String,
    pub detail: String,
}

impl ArgvObservation {
    pub fn new(argv: &[&str], verdict: &ArgvVerdict) -> Self {
        let detail = match verdict {
            ArgvVerdict::Admitted { rule } => rule.clone(),
            ArgvVerdict::WouldDeny { reason } => reason.clone(),
        };
        Self {
            schema: ARGV_OBSERVATION_SCHEMA.to_string(),
            argv: argv.iter().map(|s| (*s).to_string()).collect(),
            verdict: verdict.label().to_string(),
            detail,
        }
    }
}

/// Canonical build-gate actions: `make:*` operator intents expand to the exact
/// nested cargo gates the builder may execute. Pure mapping owned by the argv
/// admission domain so build scheduling and source release share one source.
pub fn canonical_build_actions(requested_action: &str) -> Result<Vec<String>, String> {
    match requested_action {
        "make:check" => Ok(["make:check", "cargo:check", "cargo:run", "cargo:test"]
            .into_iter()
            .map(str::to_string)
            .collect()),
        "make:build" => Ok(["make:build", "cargo:build"]
            .into_iter()
            .map(str::to_string)
            .collect()),
        "make:build-release" => Ok(["make:build-release", "cargo:build"]
            .into_iter()
            .map(str::to_string)
            .collect()),
        _ => Err(format!(
            "unsupported canonical build action: {requested_action}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(program: &str, prefix: &[&str]) -> AllowRule {
        AllowRule {
            program: program.to_string(),
            arg_prefix: prefix.iter().map(|s| (*s).to_string()).collect(),
            reason: "test".to_string(),
        }
    }

    #[test]
    fn empty_ruleset_denies_but_names_program() {
        let verdict = observe_argv_verdict(&["git", "status"], &[]);
        assert_eq!(
            verdict,
            ArgvVerdict::WouldDeny {
                reason: "no rule admits program \"git\"".to_string()
            }
        );
    }

    #[test]
    fn empty_argv_denies() {
        let verdict = observe_argv_verdict(&[], &[rule("git", &[])]);
        assert!(!verdict.admitted());
    }

    #[test]
    fn bare_program_rule_admits_any_args() {
        let rules = vec![rule("df", &[])];
        assert!(observe_argv_verdict(&["df", "-k", "/"], &rules).admitted());
    }

    #[test]
    fn prefix_rule_admits_matching_prefix_only() {
        let rules = vec![rule("cargo", &["check"])];
        assert!(observe_argv_verdict(&["cargo", "check", "-p", "x"], &rules).admitted());
        assert!(!observe_argv_verdict(&["cargo", "build"], &rules).admitted());
    }

    #[test]
    fn short_argv_does_not_match_longer_prefix() {
        let rules = vec![rule("cargo", &["check", "-p"])];
        assert!(!observe_argv_verdict(&["cargo", "check"], &rules).admitted());
    }

    #[test]
    fn observation_names_verdict_and_detail() {
        let verdict = observe_argv_verdict(&["rm", "-rf", "/x"], &[]);
        let obs = ArgvObservation::new(&["rm", "-rf", "/x"], &verdict);
        assert_eq!(obs.schema, ARGV_OBSERVATION_SCHEMA);
        assert_eq!(obs.verdict, "would_deny");
        assert_eq!(obs.argv, vec!["rm", "-rf", "/x"]);
    }
}
