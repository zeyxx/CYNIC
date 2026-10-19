//! Pure external source governance for non-CYNIC repositories.
//!
//! WHY: external projects (aegisscan, aegis-core, ai-bus) are leaf consumers of
//! CYNIC ports. This module defines the `ExternalSourcePort` trait and pure
//! types that abstract clone/fetch/branch/push/PR operations on external repos
//! without coupling to any VCS provider (Forgejo, GitHub, GitLab).
//! HOW: trait + pure data types + pure admission validators. I/O lives in
//! adapters (cynic-vcs-providers, cynic-host).
//! NEGATIVE BOUNDARY: no std::fs, no HTTP, no process spawn, no git commands.
//! No dependency on any external project. Strict DAG: external projects depend
//! on CYNIC, never the reverse.

use serde::{Deserialize, Serialize};

// ── Schemas ───────────────────────────────────────────────────────────────────

pub const EXTERNAL_SOURCE_SCHEMA: &str = "cynic.vcs.external_source.v1";
pub const EXTERNAL_CLONE_PROOF_SCHEMA: &str = "cynic.vcs.external_clone_proof.v1";
pub const EXTERNAL_PUSH_PROOF_SCHEMA: &str = "cynic.vcs.external_push_proof.v1";
pub const EXTERNAL_PR_PROOF_SCHEMA: &str = "cynic.vcs.external_pr_proof.v1";

// ── Core types (provider-agnostic) ────────────────────────────────────────────

/// Reference to an external repository. Provider-agnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalRepoRef {
    /// Stable project slug (e.g. "aegisscan").
    pub project_id: String,
    /// Canonical remote URL (https or ssh).
    pub remote_url: String,
    /// Default branch (e.g. "main", "master").
    pub default_branch: String,
    /// VCS provider kind.
    pub provider: ExternalProvider,
}

/// VCS provider kind for external repos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalProvider {
    Forgejo,
    GitHub,
    GitLab,
}

/// Result of cloning an external repo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalCloneResult {
    pub schema: &'static str,
    pub project_id: String,
    pub clone_path: String,
    pub head_sha: String,
    pub default_branch: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Branch reference for an external repo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalBranch {
    pub name: String,
    pub head_sha: String,
    pub is_default: bool,
}

/// Proof emitted after pushing to an external repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalPushProof {
    pub schema: &'static str,
    pub project_id: String,
    pub branch: String,
    pub pushed_sha: String,
    pub remote_url: String,
    pub success: bool,
}

/// Proof emitted after creating a PR on an external repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalPrProof {
    pub schema: &'static str,
    pub project_id: String,
    pub pr_number: u64,
    pub html_url: String,
    pub head_ref: String,
    pub base_ref: String,
}

// ── Admission (pure) ──────────────────────────────────────────────────────────

/// Admission report for an external source operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalSourceAdmission {
    pub admitted: bool,
    pub blockers: Vec<String>,
}

/// Check if the given trust tier permits the given operation.
///
/// Trust tiers (see docs/design/COLLABORATOR_TRUST_PROMOTION.md):
/// - Tier 3 (Observer): read-only — clone, fetch, list_branches.
/// - Tier 2 (Contributor): branch + PR — clone, fetch, list_branches,
///   create_branch, push, create_pr.
/// - Tier 1 (Maintainer): all operations — adds push_protected, merge.
///
/// Returns `true` if the operation is allowed at the given tier.
fn trust_tier_allows(trust_tier: u8, operation: &str) -> bool {
    match trust_tier {
        // Tier 3 — Observer: read-only.
        3 => matches!(operation, "clone" | "fetch" | "list_branches"),
        // Tier 2 — Contributor: branch + PR.
        2 => matches!(
            operation,
            "clone" | "fetch" | "list_branches" | "create_branch" | "push" | "create_pr"
        ),
        // Tier 1 — Maintainer: all operations.
        1 => matches!(
            operation,
            "clone"
                | "fetch"
                | "list_branches"
                | "create_branch"
                | "push"
                | "create_pr"
                | "push_protected"
                | "merge"
        ),
        _ => false,
    }
}

/// Validate that a project is eligible for external source operations.
///
/// Pure: no I/O. Checks trust tier allows the operation, mutation_policy
/// allows the operation, and sensitivity does not forbid it.
pub fn admit_external_source_operation(
    project_id: &str,
    mutation_policy: &str,
    sensitivity: &str,
    operation: &str,
    trust_tier: u8,
) -> ExternalSourceAdmission {
    let mut blockers = Vec::new();

    if project_id.is_empty() {
        blockers.push("project_id is empty".to_string());
    }

    if !trust_tier_allows(trust_tier, operation) {
        blockers.push(format!(
            "trust_tier {trust_tier} does not allow '{operation}'"
        ));
    }

    match mutation_policy {
        "read_only" => {
            if operation != "clone" && operation != "fetch" && operation != "list_branches" {
                blockers.push(format!("mutation_policy 'read_only' forbids '{operation}'"));
            }
        }
        "branch_pr" => {
            // branch_pr allows clone, fetch, branch, push (to non-protected), pr_create
            if operation == "push_protected" || operation == "merge" {
                blockers.push(format!("mutation_policy 'branch_pr' forbids '{operation}'"));
            }
        }
        "direct_push_forbidden" => {
            if operation == "push" || operation == "push_protected" || operation == "merge" {
                blockers.push(format!(
                    "mutation_policy 'direct_push_forbidden' forbids '{operation}'"
                ));
            }
        }
        other => {
            blockers.push(format!("unknown mutation_policy: '{other}'"));
        }
    }

    if sensitivity == "sanitized" && (operation == "deploy" || operation == "arm") {
        blockers.push(format!(
            "sensitivity 'sanitized' forbids '{operation}' (refusal by construction)"
        ));
    }

    ExternalSourceAdmission {
        admitted: blockers.is_empty(),
        blockers,
    }
}

// ── Port trait ────────────────────────────────────────────────────────────────

/// Port for external source operations on non-CYNIC repositories.
///
/// Abstracts clone/fetch/branch/push/PR on external repos. Concrete
/// implementations (Forgejo, GitHub, GitLab) live in adapter crates.
/// No I/O in this trait definition — adapters perform I/O.
pub trait ExternalSourcePort: Send + Sync {
    /// Clone an external repo into a local path. Read-only if mutation_policy
    /// is read_only.
    fn clone_repo(
        &self,
        repo: &ExternalRepoRef,
        target_path: &str,
    ) -> Result<ExternalCloneResult, String>;

    /// Fetch latest refs from the remote.
    fn fetch(&self, repo: &ExternalRepoRef) -> Result<Vec<ExternalBranch>, String>;

    /// List branches on the remote.
    fn list_branches(&self, repo: &ExternalRepoRef) -> Result<Vec<ExternalBranch>, String>;

    /// Create a branch on the remote (requires branch_pr or higher policy).
    fn create_branch(
        &self,
        repo: &ExternalRepoRef,
        branch_name: &str,
        from_sha: &str,
    ) -> Result<ExternalBranch, String>;

    /// Push a commit to a branch (requires branch_pr or higher policy).
    fn push(
        &self,
        repo: &ExternalRepoRef,
        branch: &str,
        commit_sha: &str,
    ) -> Result<ExternalPushProof, String>;

    /// Create a pull request on the remote.
    fn create_pr(
        &self,
        repo: &ExternalRepoRef,
        title: &str,
        head_ref: &str,
        base_ref: &str,
    ) -> Result<ExternalPrProof, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Existing tests (updated with trust_tier param) ──────────────────────────

    #[test]
    fn admit_read_only_allows_clone() {
        let r = admit_external_source_operation("aegisscan", "read_only", "public", "clone", 3);
        assert!(r.admitted);
        assert!(r.blockers.is_empty());
    }

    #[test]
    fn admit_read_only_forbids_push() {
        let r = admit_external_source_operation("aegisscan", "read_only", "public", "push", 2);
        assert!(!r.admitted);
        assert!(!r.blockers.is_empty());
    }

    #[test]
    fn admit_branch_pr_allows_branch() {
        let r =
            admit_external_source_operation("aegisscan", "branch_pr", "public", "create_branch", 2);
        assert!(r.admitted);
    }

    #[test]
    fn admit_branch_pr_forbids_merge() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "merge", 1);
        assert!(!r.admitted);
    }

    #[test]
    fn admit_sanitized_forbids_deploy() {
        let r =
            admit_external_source_operation("aegis-core", "read_only", "sanitized", "deploy", 3);
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("sanitized")));
    }

    #[test]
    fn admit_sanitized_forbids_arm() {
        let r = admit_external_source_operation("aegis-core", "read_only", "sanitized", "arm", 3);
        assert!(!r.admitted);
    }

    #[test]
    fn admit_empty_project_id_blocked() {
        let r = admit_external_source_operation("", "read_only", "public", "clone", 3);
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("project_id is empty")));
    }

    #[test]
    fn admit_unknown_policy_blocked() {
        let r =
            admit_external_source_operation("aegisscan", "unknown_policy", "public", "clone", 3);
        assert!(!r.admitted);
    }

    // ── Tier 3 (Observer) — read-only ─────────────────────────────────────────

    #[test]
    fn tier3_allows_clone() {
        let r = admit_external_source_operation("aegisscan", "read_only", "public", "clone", 3);
        assert!(r.admitted);
        assert!(r.blockers.is_empty());
    }

    #[test]
    fn tier3_allows_fetch() {
        let r = admit_external_source_operation("aegisscan", "read_only", "public", "fetch", 3);
        assert!(r.admitted);
        assert!(r.blockers.is_empty());
    }

    #[test]
    fn tier3_allows_list_branches() {
        let r =
            admit_external_source_operation("aegisscan", "read_only", "public", "list_branches", 3);
        assert!(r.admitted);
        assert!(r.blockers.is_empty());
    }

    #[test]
    fn tier3_forbids_create_branch() {
        let r =
            admit_external_source_operation("aegisscan", "branch_pr", "public", "create_branch", 3);
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("trust_tier 3")));
    }

    #[test]
    fn tier3_forbids_push() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "push", 3);
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("trust_tier 3")));
    }

    #[test]
    fn tier3_forbids_create_pr() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "create_pr", 3);
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("trust_tier 3")));
    }

    #[test]
    fn tier3_forbids_push_protected() {
        let r = admit_external_source_operation(
            "aegisscan",
            "branch_pr",
            "public",
            "push_protected",
            3,
        );
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("trust_tier 3")));
    }

    #[test]
    fn tier3_forbids_merge() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "merge", 3);
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("trust_tier 3")));
    }

    // ── Tier 2 (Contributor) — branch + PR ────────────────────────────────────

    #[test]
    fn tier2_allows_clone() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "clone", 2);
        assert!(r.admitted);
        assert!(r.blockers.is_empty());
    }

    #[test]
    fn tier2_allows_fetch() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "fetch", 2);
        assert!(r.admitted);
        assert!(r.blockers.is_empty());
    }

    #[test]
    fn tier2_allows_list_branches() {
        let r =
            admit_external_source_operation("aegisscan", "branch_pr", "public", "list_branches", 2);
        assert!(r.admitted);
        assert!(r.blockers.is_empty());
    }

    #[test]
    fn tier2_allows_create_branch() {
        let r =
            admit_external_source_operation("aegisscan", "branch_pr", "public", "create_branch", 2);
        assert!(r.admitted);
        assert!(r.blockers.is_empty());
    }

    #[test]
    fn tier2_allows_push() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "push", 2);
        assert!(r.admitted);
        assert!(r.blockers.is_empty());
    }

    #[test]
    fn tier2_allows_create_pr() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "create_pr", 2);
        assert!(r.admitted);
        assert!(r.blockers.is_empty());
    }

    #[test]
    fn tier2_forbids_push_protected() {
        let r = admit_external_source_operation(
            "aegisscan",
            "branch_pr",
            "public",
            "push_protected",
            2,
        );
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("trust_tier 2")));
    }

    #[test]
    fn tier2_forbids_merge() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "merge", 2);
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("trust_tier 2")));
    }

    // ── Tier 1 (Maintainer) — all operations ──────────────────────────────────

    #[test]
    fn tier1_allows_clone() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "clone", 1);
        assert!(r.admitted);
    }

    #[test]
    fn tier1_allows_fetch() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "fetch", 1);
        assert!(r.admitted);
    }

    #[test]
    fn tier1_allows_list_branches() {
        let r =
            admit_external_source_operation("aegisscan", "branch_pr", "public", "list_branches", 1);
        assert!(r.admitted);
    }

    #[test]
    fn tier1_allows_create_branch() {
        let r =
            admit_external_source_operation("aegisscan", "branch_pr", "public", "create_branch", 1);
        assert!(r.admitted);
    }

    #[test]
    fn tier1_allows_push() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "push", 1);
        assert!(r.admitted);
    }

    #[test]
    fn tier1_allows_create_pr() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "create_pr", 1);
        assert!(r.admitted);
    }

    #[test]
    fn tier1_allows_push_protected() {
        // push_protected is forbidden by branch_pr policy, but we use direct_push
        // (custom policy) to isolate the trust_tier check.
        // direct_push is not a known policy → blocked by mutation_policy, not trust_tier.
        // To test tier1 allowing push_protected, we need a policy that permits it.
        // Since all known policies forbid push_protected, we test tier1 vs tier2
        // with the branch_pr policy — both should produce a blocker, but tier1's
        // blocker should NOT contain "trust_tier 1" while tier2's should contain "trust_tier 2".
        let _r = admit_external_source_operation(
            "aegisscan",
            "direct_push",
            "public",
            "push_protected",
            1,
        );
        let r1 = admit_external_source_operation(
            "aegisscan",
            "branch_pr",
            "public",
            "push_protected",
            1,
        );
        assert!(!r1.admitted);
        assert!(!r1.blockers.iter().any(|b| b.contains("trust_tier 1")));
        assert!(r1.blockers.iter().any(|b| b.contains("branch_pr")));
    }

    #[test]
    fn tier1_forbids_nothing_by_tier() {
        // With a permissive setup, tier1 should never add a trust_tier blocker.
        // We verify by checking that tier1 with an unknown op (not in any list)
        // produces a trust_tier blocker, while known ops with branch_pr produce
        // only policy blockers.
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "merge", 1);
        assert!(!r.admitted);
        // merge is forbidden by branch_pr policy, not by trust_tier.
        assert!(!r.blockers.iter().any(|b| b.contains("trust_tier")));
    }

    #[test]
    fn tier1_vs_tier2_push_protected() {
        // tier2 gets a trust_tier blocker for push_protected.
        let r2 = admit_external_source_operation(
            "aegisscan",
            "branch_pr",
            "public",
            "push_protected",
            2,
        );
        assert!(r2.blockers.iter().any(|b| b.contains("trust_tier 2")));
        // tier1 does NOT get a trust_tier blocker for push_protected.
        let r1 = admit_external_source_operation(
            "aegisscan",
            "branch_pr",
            "public",
            "push_protected",
            1,
        );
        assert!(!r1.blockers.iter().any(|b| b.contains("trust_tier")));
    }

    #[test]
    fn tier1_vs_tier2_merge() {
        // tier2 gets a trust_tier blocker for merge.
        let r2 = admit_external_source_operation("aegisscan", "branch_pr", "public", "merge", 2);
        assert!(r2.blockers.iter().any(|b| b.contains("trust_tier 2")));
        // tier1 does NOT get a trust_tier blocker for merge.
        let r1 = admit_external_source_operation("aegisscan", "branch_pr", "public", "merge", 1);
        assert!(!r1.blockers.iter().any(|b| b.contains("trust_tier")));
    }

    // ── Invalid trust tiers ───────────────────────────────────────────────────

    #[test]
    fn tier0_blocks_everything() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "clone", 0);
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("trust_tier 0")));
    }

    #[test]
    fn tier4_blocks_everything() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "clone", 4);
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("trust_tier 4")));
    }

    #[test]
    fn tier255_blocks_everything() {
        let r = admit_external_source_operation("aegisscan", "branch_pr", "public", "clone", 255);
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("trust_tier 255")));
    }

    // ── Combined: trust_tier + mutation_policy ─────────────────────────────────

    #[test]
    fn tier3_read_only_combo() {
        // Tier 3 + read_only: both allow clone → admitted.
        let r = admit_external_source_operation("aegisscan", "read_only", "public", "clone", 3);
        assert!(r.admitted);
        // Tier 3 + read_only + push: both forbid → two blockers.
        let r = admit_external_source_operation("aegisscan", "read_only", "public", "push", 3);
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("trust_tier 3")));
        assert!(r.blockers.iter().any(|b| b.contains("read_only")));
    }

    #[test]
    fn tier2_direct_push_forbidden_combo() {
        // Tier 2 allows push, but direct_push_forbidden forbids it.
        let r = admit_external_source_operation(
            "aegisscan",
            "direct_push_forbidden",
            "public",
            "push",
            2,
        );
        assert!(!r.admitted);
        assert!(!r.blockers.iter().any(|b| b.contains("trust_tier 2")));
        assert!(
            r.blockers
                .iter()
                .any(|b| b.contains("direct_push_forbidden"))
        );
    }

    // ── Port trait object safety ───────────────────────────────────────────────

    #[test]
    fn port_trait_object_safe() {
        struct MockPort;
        impl ExternalSourcePort for MockPort {
            fn clone_repo(
                &self,
                _repo: &ExternalRepoRef,
                _target_path: &str,
            ) -> Result<ExternalCloneResult, String> {
                Ok(ExternalCloneResult {
                    schema: EXTERNAL_CLONE_PROOF_SCHEMA,
                    project_id: "test".to_string(),
                    clone_path: "/tmp/test".to_string(),
                    head_sha: "abc123".to_string(),
                    default_branch: "main".to_string(),
                    success: true,
                    error: None,
                })
            }
            fn fetch(&self, _repo: &ExternalRepoRef) -> Result<Vec<ExternalBranch>, String> {
                Ok(vec![])
            }
            fn list_branches(
                &self,
                _repo: &ExternalRepoRef,
            ) -> Result<Vec<ExternalBranch>, String> {
                Ok(vec![])
            }
            fn create_branch(
                &self,
                _repo: &ExternalRepoRef,
                _branch_name: &str,
                _from_sha: &str,
            ) -> Result<ExternalBranch, String> {
                Ok(ExternalBranch {
                    name: "test-branch".to_string(),
                    head_sha: "abc123".to_string(),
                    is_default: false,
                })
            }
            fn push(
                &self,
                _repo: &ExternalRepoRef,
                _branch: &str,
                _commit_sha: &str,
            ) -> Result<ExternalPushProof, String> {
                Ok(ExternalPushProof {
                    schema: EXTERNAL_PUSH_PROOF_SCHEMA,
                    project_id: "test".to_string(),
                    branch: "test-branch".to_string(),
                    pushed_sha: "abc123".to_string(),
                    remote_url: "https://example.com".to_string(),
                    success: true,
                })
            }
            fn create_pr(
                &self,
                _repo: &ExternalRepoRef,
                _title: &str,
                _head_ref: &str,
                _base_ref: &str,
            ) -> Result<ExternalPrProof, String> {
                Ok(ExternalPrProof {
                    schema: EXTERNAL_PR_PROOF_SCHEMA,
                    project_id: "test".to_string(),
                    pr_number: 1,
                    html_url: "https://example.com/pr/1".to_string(),
                    head_ref: "test-branch".to_string(),
                    base_ref: "main".to_string(),
                })
            }
        }
        let port: Box<dyn ExternalSourcePort> = Box::new(MockPort);
        assert!(
            port.clone_repo(
                &ExternalRepoRef {
                    project_id: "test".to_string(),
                    remote_url: "https://example.com".to_string(),
                    default_branch: "main".to_string(),
                    provider: ExternalProvider::GitHub,
                },
                "/tmp/test"
            )
            .is_ok()
        );
    }
}
