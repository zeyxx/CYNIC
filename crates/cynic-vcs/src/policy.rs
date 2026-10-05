//! Source governance policy — pure decision layer for the agent-source protocol.
//!
//! Merges the former `source_policy` (commit/branch conventions, protected refs)
//! with the source-change admission rules of `source_control`. Zero I/O: given a
//! value and a policy, each function returns blocker codes.

use cynic_protocol::SecurityEvidenceState;
use cynic_protocol::vcs::SourceSnapshot;
use serde::{Deserialize, Serialize};

use crate::types::{
    SOURCE_ADMISSION_REPORT_SCHEMA, SOURCE_CHANGE_PROPOSAL_SCHEMA, SourceChangeAdmission,
    SourceChangeProposal, SourceMatterClass, SourcePromotionStage, SourceStageAdmissionReport,
};

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
pub const ACCEPTED_BRANCH_PREFIXES: &[&str] = &[
    "agent",
    "integration",
    "recovery",
    "backup",
    "feat",
    "fix",
    "docs",
    "refactor",
    "test",
    "chore",
];

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
            accepted_types: ACCEPTED_COMMIT_TYPES
                .iter()
                .map(|s| s.to_string())
                .collect(),
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
            accepted_prefixes: ACCEPTED_BRANCH_PREFIXES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            protected_refs: DEFAULT_PROTECTED_REFS
                .iter()
                .map(|s| s.to_string())
                .collect(),
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
    if !policy
        .accepted_types
        .iter()
        .any(|t| t.eq_ignore_ascii_case(&ctype))
    {
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

fn is_sensitive_source_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let mut components = normalized.split('/').filter(|part| !part.is_empty());
    let Some(file_name) = components.next_back() else {
        return false;
    };
    let file_name = file_name.to_ascii_lowercase();

    if components.any(|part| {
        matches!(
            part.to_ascii_lowercase().as_str(),
            "secrets" | ".secrets" | "private-keys"
        )
    }) {
        return true;
    }

    file_name == ".env"
        || file_name.starts_with(".env.")
        || matches!(
            file_name.as_str(),
            "credentials.json"
                | "credentials.toml"
                | "id_ed25519"
                | "id_rsa"
                | "token"
                | "token.txt"
        )
        || [".key", ".pem", ".p12", ".pfx"]
            .iter()
            .any(|suffix| file_name.ends_with(suffix))
}

pub fn admit_source_change(proposal: &SourceChangeProposal) -> SourceChangeAdmission {
    let mut blockers = Vec::new();
    if proposal.schema != SOURCE_CHANGE_PROPOSAL_SCHEMA {
        blockers.push("proposal_schema_mismatch".to_string());
    }
    if proposal.proposal_id.trim().is_empty() {
        blockers.push("proposal_id_missing".to_string());
    }
    if proposal.repository_id.trim().is_empty() {
        blockers.push("repository_id_missing".to_string());
    }
    if proposal.worktree_id.trim().is_empty() {
        blockers.push("worktree_id_missing".to_string());
    }
    if proposal.lease_id.trim().is_empty() {
        blockers.push("worktree_lease_id_missing".to_string());
    }
    if proposal.fencing_token == 0 {
        blockers.push("worktree_fencing_token_missing".to_string());
    }
    if proposal.worktree_id.contains('/') || proposal.worktree_id.contains('\\') {
        blockers.push("worktree_id_not_path_safe".to_string());
    }
    if proposal.base_commit.trim().is_empty() {
        blockers.push("base_commit_missing".to_string());
    }
    if proposal.actor.trim().is_empty() || proposal.node_id.trim().is_empty() {
        blockers.push("change_identity_incomplete".to_string());
    }
    if proposal.intent.trim().is_empty() {
        blockers.push("change_intent_missing".to_string());
    }
    if proposal.paths.is_empty() {
        blockers.push("change_paths_missing".to_string());
    }
    if proposal.rollback.trim().is_empty() {
        blockers.push("rollback_intent_missing".to_string());
    }
    if proposal.validation != SecurityEvidenceState::Proven {
        blockers.push("validation_unproven".to_string());
    }
    // Source-governance: commit intent must follow the agent-source protocol.
    // `intent` is the commit subject; an agent that cannot express its change
    // as a Conventional Commit has not met the contract.
    let commit_blockers = commit_conforms(&proposal.intent, &CommitPolicy::default());
    blockers.extend(commit_blockers);
    match proposal.matter {
        SourceMatterClass::Secret => blockers.push("secret_matter_never_versioned".to_string()),
        SourceMatterClass::GeneratedArtifact => {
            blockers.push("generated_artifact_requires_release_store".to_string())
        }
        SourceMatterClass::Genome
        | SourceMatterClass::DesiredState
        | SourceMatterClass::Adapter
        | SourceMatterClass::Evidence => {}
    }
    for path in &proposal.paths {
        if path.starts_with('/') || path.split('/').any(|part| part == "..") {
            blockers.push(format!("path_outside_repository:{path}"));
        }
        if is_sensitive_source_path(path) {
            blockers.push(format!("sensitive_path_not_versioned:{path}"));
        }
    }
    SourceChangeAdmission {
        schema: "cynic.source.change_admission.v1".to_string(),
        status: if blockers.is_empty() {
            "admitted"
        } else {
            "blocked"
        }
        .to_string(),
        proposal_id: proposal.proposal_id.clone(),
        blockers,
        evidence_refs: proposal.evidence_refs.clone(),
    }
}

pub fn admit_source_stage(
    snapshot: &SourceSnapshot,
    target_stage: SourcePromotionStage,
) -> SourceStageAdmissionReport {
    let mut blockers = Vec::new();
    if snapshot.branch.as_deref().is_none_or(str::is_empty) {
        blockers.push("branch_identity_missing".to_string());
    }
    if snapshot.head_commit.as_deref().is_none_or(str::is_empty) {
        blockers.push("head_commit_missing".to_string());
    }
    if !snapshot.unmerged_paths.is_empty() {
        blockers.push("unmerged_paths_present".to_string());
    }

    match target_stage {
        SourcePromotionStage::Worktree => {}
        SourcePromotionStage::CommitCandidate => {
            if snapshot.staged_paths.is_empty() {
                blockers.push("explicit_staged_path_set_required".to_string());
            }
            if snapshot.validation != SecurityEvidenceState::Proven {
                blockers.push("validation_unproven".to_string());
            }
        }
        SourcePromotionStage::SovereignCommit => {
            if !snapshot.staged_paths.is_empty()
                || !snapshot.unstaged_paths.is_empty()
                || !snapshot.untracked_paths.is_empty()
            {
                blockers.push("worktree_not_clean".to_string());
            }
            if snapshot.validation != SecurityEvidenceState::Proven {
                blockers.push("validation_unproven".to_string());
            }
            if snapshot.sovereign_transport_identity != SecurityEvidenceState::Proven {
                blockers.push("sovereign_transport_identity_unproven".to_string());
            }
            if snapshot.sovereign_authentication != SecurityEvidenceState::Proven {
                blockers.push("sovereign_authentication_unproven".to_string());
            }
            if snapshot.remote_commit_matches != SecurityEvidenceState::Proven {
                blockers.push("remote_commit_match_unproven".to_string());
            }
            // Source-governance: an agent may only promote to a branch it owns
            // (`agent/<id>/<scope>`), never to a protected trunk directly.
            if let Some(branch) = snapshot.branch.as_deref() {
                let policy = BranchPolicy::default();
                let branch_blockers = branch_conforms(branch, &policy);
                blockers.extend(branch_blockers);
                if ref_is_protected(branch, &policy.protected_refs) {
                    blockers.push("sovereign_promotion_to_protected_ref".to_string());
                }
            } else {
                blockers.push("branch_identity_missing".to_string());
            }
        }
        SourcePromotionStage::ReleaseSource => {
            if snapshot.remote_commit_matches != SecurityEvidenceState::Proven {
                blockers.push("remote_commit_match_unproven".to_string());
            }
            if snapshot
                .build_input_digest
                .as_deref()
                .is_none_or(str::is_empty)
            {
                blockers.push("build_input_digest_missing".to_string());
            }
            if snapshot.evidence_refs.is_empty() {
                blockers.push("source_evidence_missing".to_string());
            }
        }
    }

    SourceStageAdmissionReport {
        schema: SOURCE_ADMISSION_REPORT_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            "admitted".to_string()
        } else {
            "blocked".to_string()
        },
        target_stage,
        blockers,
        evidence_refs: snapshot.evidence_refs.clone(),
    }
}

#[cfg(test)]
mod source_policy_tests {
    use super::*;

    #[test]
    fn conventional_commit_conforms() {
        let policy = CommitPolicy::default();
        assert!(
            commit_conforms("feat(protocol): add source policy predicates", &policy).is_empty()
        );
        assert!(commit_conforms("fix(source): reject direct main push", &policy).is_empty());
        assert!(commit_conforms("chore: regenerate Cargo.lock", &policy).is_empty());
        assert!(commit_conforms("refactor!: drop legacy git adapter", &policy).is_empty());
    }

    #[test]
    fn non_conventional_commit_is_blocked() {
        let policy = CommitPolicy::default();
        let blockers = commit_conforms("updated the thing", &policy);
        assert!(
            blockers
                .iter()
                .any(|b| b == "commit_intent_not_conventional")
        );
    }

    #[test]
    fn unknown_commit_type_is_blocked() {
        let policy = CommitPolicy::default();
        let blockers = commit_conforms("wip: half done", &policy);
        assert!(
            blockers
                .iter()
                .any(|b| b.starts_with("commit_type_unaccepted"))
        );
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
        assert!(
            blockers
                .iter()
                .any(|b| b.starts_with("branch_prefix_unaccepted"))
        );
    }

    #[test]
    fn unsafe_branch_chars_blocked() {
        let policy = BranchPolicy::default();
        let blockers = branch_conforms("agent/hermes/foo bar", &policy);
        assert!(blockers.iter().any(|b| b == "branch_name_unsafe_chars"));
    }

    #[test]
    fn ref_is_protected_detects_trunk() {
        let protected: Vec<String> = DEFAULT_PROTECTED_REFS
            .iter()
            .map(|s| s.to_string())
            .collect();
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
        assert_eq!(
            agent_branch_for("hermes", ""),
            Some("agent/hermes/work".to_string())
        );
        assert_eq!(agent_branch_for("", "x"), None);
    }

    #[test]
    fn forbidden_substring_in_commit_is_blocked() {
        let mut policy = CommitPolicy::default();
        policy.forbidden_substrings.push("PASSWORD=".to_string());
        let blockers = commit_conforms("fix(auth): rotate PASSWORD=supersecret", &policy);
        assert!(
            blockers
                .iter()
                .any(|b| b.starts_with("commit_intent_contains_forbidden"))
        );
    }
}
#[cfg(test)]
mod source_control_tests {
    use super::*;

    #[test]
    fn dirty_worktree_cannot_be_declared_sovereign() {
        let snapshot = SourceSnapshot {
            branch: Some("feature".to_string()),
            head_commit: Some("abc123".to_string()),
            upstream: Some("origin/feature".to_string()),
            staged_paths: Vec::new(),
            unstaged_paths: vec!["src/lib.rs".to_string()],
            untracked_paths: vec!["local-proof.json".to_string()],
            unmerged_paths: Vec::new(),
            validation: SecurityEvidenceState::Proven,
            sovereign_transport_identity: SecurityEvidenceState::Absent,
            sovereign_authentication: SecurityEvidenceState::Unknown,
            remote_commit_matches: SecurityEvidenceState::Unknown,
            build_input_digest: None,
            evidence_refs: Vec::new(),
        };

        let report = admit_source_stage(&snapshot, SourcePromotionStage::SovereignCommit);

        assert_eq!(report.status, "blocked");
        assert!(report.blockers.contains(&"worktree_not_clean".to_string()));
        assert!(
            report
                .blockers
                .contains(&"sovereign_transport_identity_unproven".to_string())
        );
    }

    #[test]
    fn secrets_and_generated_artifacts_cannot_enter_source_history() {
        let proposal = SourceChangeProposal {
            schema: SOURCE_CHANGE_PROPOSAL_SCHEMA.to_string(),
            proposal_id: "p-1".to_string(),
            repository_id: "cynic".to_string(),
            worktree_id: "agent-a".to_string(),
            lease_id: "lease-1".to_string(),
            fencing_token: 1,
            base_commit: "abc123".to_string(),
            actor: "agent-a".to_string(),
            node_id: "node-a".to_string(),
            intent: "checkpoint".to_string(),
            matter: SourceMatterClass::GeneratedArtifact,
            paths: vec![".env".to_string(), "target/release/cynic".to_string()],
            validation: SecurityEvidenceState::Proven,
            rollback: "revert proposal p-1".to_string(),
            evidence_refs: vec!["proof-1".to_string()],
        };
        let report = admit_source_change(&proposal);
        assert_eq!(report.status, "blocked");
        assert!(
            report
                .blockers
                .iter()
                .any(|v| v == "sensitive_path_not_versioned:.env")
        );
        assert!(
            report
                .blockers
                .iter()
                .any(|v| v == "generated_artifact_requires_release_store")
        );
    }

    #[test]
    fn secret_handling_source_names_are_not_secret_material() {
        for path in [
            "crates/cynic-host/src/runtime_secret.rs",
            "crates/cynic-protocol/src/secret_material.rs",
            "crates/cynic-host/src/secret_lint",
            "crates/cynic-protocol/src/token_policy.rs",
        ] {
            assert!(
                !is_sensitive_source_path(path),
                "unexpected rejection: {path}"
            );
        }
    }

    #[test]
    fn secret_material_paths_are_rejected_structurally() {
        for path in [
            ".env.local",
            "config/credentials.json",
            "infra/secrets/runtime.toml",
            "keys/node.key",
            "keys/node.PEM",
            "private-keys/node.pub",
            "bootstrap/token.txt",
        ] {
            assert!(
                is_sensitive_source_path(path),
                "unexpected admission: {path}"
            );
        }
    }

    #[test]
    fn a_valid_source_change_is_admitted_before_git() {
        let proposal = SourceChangeProposal {
            schema: SOURCE_CHANGE_PROPOSAL_SCHEMA.to_string(),
            proposal_id: "p-2".to_string(),
            repository_id: "cynic".to_string(),
            worktree_id: "agent-a".to_string(),
            lease_id: "lease-2".to_string(),
            fencing_token: 2,
            base_commit: "abc123".to_string(),
            actor: "agent-a".to_string(),
            node_id: "node-a".to_string(),
            intent: "feat(network): add observation proof".to_string(),
            matter: SourceMatterClass::Adapter,
            paths: vec!["crates/cynic-node/src/observe.rs".to_string()],
            validation: SecurityEvidenceState::Proven,
            rollback: "revert commit p-2".to_string(),
            evidence_refs: vec!["conformance-1".to_string()],
        };
        let report = admit_source_change(&proposal);
        assert_eq!(report.status, "admitted");
        assert!(report.blockers.is_empty());
    }
}
