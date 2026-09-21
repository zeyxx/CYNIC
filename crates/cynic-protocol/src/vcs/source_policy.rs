//! Source governance policy — pure decision layer for the agent-source protocol.
//!
//! This module is the missing link between `agent_citizenship` (who may mutate)
//! and `source_control` (how a source change is admitted). It owns the three
//! rules that were previously only *prose* in `protocols/agent-protocol.md`
//! and never enforced in Rust:
//!
//! 1. **Commit convention** — every commit intent must follow Conventional
//!    Commits (`<type>(<scope>): <subject>`), bounded length, no secrets.
//! 2. **Branch isolation** — an agent works on `agent/<id>/<scope>` (or the
//!    other accepted prefixes), never on a shared trunk.
//! 3. **Protected refs** — `main` / `forgejo/main` / `origin/main` may only be
//!    reached through a merge bot / PR, never a direct agent push.
//!
//! All functions here are pure: given a value and a policy, return a list of
//! blocker codes. Surfaces (`cynic-cli`, MCP, kernel) project these — they must
//! not re-derive the regexes locally (AGENTS.md invariant #6).

use serde::{Deserialize, Serialize};

pub const SOURCE_POLICY_SCHEMA: &str = "cynic.source_policy.v1";

/// Conventional-commits type vocabulary accepted by the protocol.
///
/// Keep in sync with `infra/agent-source-protocol.json`. `!:` (breaking) is
/// permitted by the regex but is always treated as a review-gated change.
pub const ACCEPTED_COMMIT_TYPES: &[&str] = &[
    "feat", "fix", "test", "docs", "refactor", "chore", "ci", "build", "perf", "style",
];

/// Prefixes of branches that an agent may claim as its isolated work surface.
///
/// `agent` is the canonical per-agent prefix (`agent/<id>/<scope>`). The others
/// are retained for human/infra workflows that still want isolation.
pub const ACCEPTED_BRANCH_PREFIXES: &[&str] =
    &["agent", "integration", "recovery", "backup", "feat", "fix", "docs", "refactor", "test", "chore"];

/// Refs that no agent may push to directly. Promotion to these refs happens
/// only through the merge bot / PR flow.
pub const DEFAULT_PROTECTED_REFS: &[&str] = &["main", "forgejo/main", "origin/main", "master"];

/// Bounds for commit intent validation.
pub const MAX_COMMIT_SUBJECT_LEN: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitPolicy {
    pub schema: String,
    /// Accepted Conventional-Commits types (lower-case).
    pub accepted_types: Vec<String>,
    /// Maximum subject length (after `type(scope): `).
    pub max_subject_len: usize,
    /// Disallowed substrings (case-insensitive) — secret leakage guard.
    #[serde(default)]
    pub forbidden_substrings: Vec<String>,
}

impl Default for CommitPolicy {
    fn default() -> Self {
        Self {
            schema: SOURCE_POLICY_SCHEMA.to_string(),
            accepted_types: ACCEPTED_COMMIT_TYPES.iter().map(|s| s.to_string()).collect(),
            max_subject_len: MAX_COMMIT_SUBJECT_LEN,
            forbidden_substrings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchPolicy {
    pub schema: String,
    /// Accepted branch prefixes (lower-case, no trailing slash).
    pub accepted_prefixes: Vec<String>,
    /// Refs that no agent may push to directly.
    pub protected_refs: Vec<String>,
}

impl Default for BranchPolicy {
    fn default() -> Self {
        Self {
            schema: SOURCE_POLICY_SCHEMA.to_string(),
            accepted_prefixes: ACCEPTED_BRANCH_PREFIXES.iter().map(|s| s.to_string()).collect(),
            protected_refs: DEFAULT_PROTECTED_REFS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Validate a commit intent against the Conventional Commits policy.
///
/// Returns an empty `blockers` vec when the intent conforms. Otherwise each
/// violated rule contributes exactly one cumulative blocker code.
pub fn commit_conforms(intent: &str, policy: &CommitPolicy) -> Vec<String> {
    let mut blockers = Vec::new();
    let intent = intent.trim();

    if intent.is_empty() {
        blockers.push("commit_intent_empty".to_string());
        return blockers;
    }

    // Secret-leak guard: reject intents that embed obvious credential material.
    let lowered = intent.to_lowercase();
    for forbidden in &policy.forbidden_substrings {
        if lowered.contains(&forbidden.to_lowercase()) {
            blockers.push(format!("commit_intent_contains_forbidden:{forbidden}"));
        }
    }

    // Conventional Commits: type[(scope)][!]: subject
    // group1=type, group2=optional scope (without parens), group3=breaking "!",
    // group4=subject
    let re = match regex_light(intent) {
        Some(parsed) => parsed,
        None => {
            blockers.push("commit_intent_not_conventional".to_string());
            return blockers;
        }
    };

    let ctype = re.0.to_lowercase();
    if !policy.accepted_types.iter().any(|t| t.eq_ignore_ascii_case(&ctype)) {
        blockers.push(format!("commit_type_unaccepted:{ctype}"));
    }

    // Subject bound (the part after `<type>(scope): `).
    if let Some(subject) = re.1 {
        if subject.is_empty() {
            blockers.push("commit_subject_empty".to_string());
        } else if subject.chars().count() > policy.max_subject_len {
            blockers.push("commit_subject_too_long".to_string());
        }
    } else {
        blockers.push("commit_subject_missing".to_string());
    }

    blockers
}

/// Validate a branch name against the isolation policy.
pub fn branch_conforms(branch: &str, policy: &BranchPolicy) -> Vec<String> {
    let mut blockers = Vec::new();
    let branch = branch.trim();

    if branch.is_empty() {
        blockers.push("branch_name_empty".to_string());
        return blockers;
    }
    // No ref may be pushed to a protected trunk directly.
    if policy.protected_refs.iter().any(|r| r == branch) {
        blockers.push(format!("branch_is_protected_ref:{branch}"));
    }
    // Reject obviously unsafe characters (spaces, ~, ^, :, ?, *, [).
    if branch
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '~' | '^' | ':' | '?' | '*' | '['))
    {
        blockers.push("branch_name_unsafe_chars".to_string());
    }

    let prefix = branch.split('/').next().unwrap_or("").to_lowercase();
    if !policy.accepted_prefixes.iter().any(|p| p == &prefix) {
        blockers.push(format!("branch_prefix_unaccepted:{prefix}"));
    }

    blockers
}

/// Returns true if the ref name is protected (no direct agent push allowed).
pub fn ref_is_protected(ref_name: &str, protected: &[String]) -> bool {
    protected.iter().any(|r| r == ref_name)
}

/// Derive the canonical isolated branch name for an agent + scope.
///
/// `agent/hermes/agent-source-protocol` is the dogfooded shape: one agent,
/// one scope, one branch. Returns `None` only if `agent_id` is empty.
pub fn agent_branch_for(agent_id: &str, scope: &str) -> Option<String> {
    let agent_id = agent_id.trim();
    let scope = scope.trim();
    if agent_id.is_empty() {
        return None;
    }
    let scope = if scope.is_empty() { "work" } else { scope };
    Some(format!("agent/{agent_id}/{scope}"))
}

/// Minimal Conventional-Commits parser that avoids pulling in the `regex`
/// crate at the protocol layer (keeps `cynic-protocol` lean and offline).
///
/// Returns `(type, Option<subject>)` or `None` when the shape does not match
/// `<type>[(scope)][!]: <subject>`.
fn regex_light(intent: &str) -> Option<(&str, Option<&str>)> {
    let colon = intent.find(':')?;
    let head = &intent[..colon];
    let subject = intent[colon + 1..].trim();
    if subject.is_empty() {
        return None;
    }
    // head = `type` or `type(scope)` or `type(scope)!`
    let head = head.trim_end_matches('!');
    let (ctype, _) = if let Some(open) = head.find('(') {
        if !head.ends_with(')') {
            return None;
        }
        (&head[..open], Some(&head[open + 1..head.len() - 1]))
    } else {
        (head, None)
    };
    if ctype.is_empty() || !ctype.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
        return None;
    }
    Some((ctype, Some(subject)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conventional_commit_conforms() {
        let policy = CommitPolicy::default();
        assert!(commit_conforms("feat(protocol): add source policy predicates", &policy).is_empty());
        assert!(commit_conforms("fix(source): reject direct main push", &policy).is_empty());
        assert!(commit_conforms("chore: regenerate Cargo.lock", &policy).is_empty());
        assert!(commit_conforms("refactor!: drop legacy git adapter", &policy).is_empty());
    }

    #[test]
    fn non_conventional_commit_is_blocked() {
        let policy = CommitPolicy::default();
        let blockers = commit_conforms("updated the thing", &policy);
        assert!(blockers.iter().any(|b| b == "commit_intent_not_conventional"));
    }

    #[test]
    fn unknown_commit_type_is_blocked() {
        let policy = CommitPolicy::default();
        let blockers = commit_conforms("wip: half done", &policy);
        assert!(blockers.iter().any(|b| b.starts_with("commit_type_unaccepted")));
    }

    #[test]
    fn empty_commit_is_blocked() {
        let policy = CommitPolicy::default();
        assert!(!commit_conforms("   ", &policy).is_empty());
    }

    #[test]
    fn agent_branch_conforms() {
        let policy = BranchPolicy::default();
        assert!(branch_conforms("agent/hermes/agent-source-protocol", &policy).is_empty());
        assert!(branch_conforms("integration/github-laptop-20260816", &policy).is_empty());
        assert!(branch_conforms("recovery/federation-peer-resolver", &policy).is_empty());
    }

    #[test]
    fn main_branch_is_protected() {
        let policy = BranchPolicy::default();
        let blockers = branch_conforms("main", &policy);
        assert!(blockers.iter().any(|b| b == "branch_is_protected_ref:main"));
        assert!(blockers.iter().any(|b| b.starts_with("branch_prefix_unaccepted")));
    }

    #[test]
    fn unsafe_branch_chars_blocked() {
        let policy = BranchPolicy::default();
        let blockers = branch_conforms("agent/hermes/foo bar", &policy);
        assert!(blockers.iter().any(|b| b == "branch_name_unsafe_chars"));
    }

    #[test]
    fn ref_is_protected_detects_trunk() {
        let protected: Vec<String> = DEFAULT_PROTECTED_REFS.iter().map(|s| s.to_string()).collect();
        assert!(ref_is_protected("main", &protected));
        assert!(ref_is_protected("forgejo/main", &protected));
        assert!(!ref_is_protected("agent/hermes/x", &protected));
    }

    #[test]
    fn agent_branch_for_derives_canonical_name() {
        assert_eq!(
            agent_branch_for("hermes", "agent-source-protocol"),
            Some("agent/hermes/agent-source-protocol".to_string())
        );
        assert_eq!(agent_branch_for("hermes", ""), Some("agent/hermes/work".to_string()));
        assert_eq!(agent_branch_for("", "x"), None);
    }

    #[test]
    fn forbidden_substring_in_commit_is_blocked() {
        let mut policy = CommitPolicy::default();
        policy.forbidden_substrings.push("PASSWORD=".to_string());
        let blockers = commit_conforms("fix(auth): rotate PASSWORD=supersecret", &policy);
        assert!(blockers.iter().any(|b| b.starts_with("commit_intent_contains_forbidden")));
    }
}
