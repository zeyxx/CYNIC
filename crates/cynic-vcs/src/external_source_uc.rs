//! Pure orchestration usecases for external source operations.
//!
//! WHY: composes `ExternalSourcePort` operations into higher-level workflows
//! without coupling to any concrete VCS provider. All functions are pure
//! orchestration — they delegate I/O to the port and contain no side effects.
//! HOW: each function takes a port reference and pure data, calls port
//! methods in sequence, and returns the composed result.
//! NEGATIVE BOUNDARY: no std::fs, no HTTP, no process spawn, no git commands.

use crate::external_source::{
    ExternalBranch, ExternalCloneResult, ExternalRepoRef, ExternalSourcePort,
};

/// Orchestrate cloning a repo and creating a branch from a given SHA.
///
/// Calls `port.clone_repo` to clone the repo into a target path derived from
/// the project ID, then calls `port.create_branch` to create a branch from
/// the given `from_sha`.
///
/// Pure orchestration: no I/O in this function. All side effects are
/// delegated to the port implementation.
pub fn clone_and_branch(
    port: &impl ExternalSourcePort,
    repo: &ExternalRepoRef,
    branch_name: &str,
    from_sha: &str,
) -> Result<ExternalBranch, String> {
    let target_path = format!("/tmp/{}", repo.project_id);
    let _clone_result: ExternalCloneResult = port.clone_repo(repo, &target_path)?;
    let branch: ExternalBranch = port.create_branch(repo, branch_name, from_sha)?;
    Ok(branch)
}

/// Orchestrate cloning a repo and listing its branches.
///
/// Calls `port.clone_repo` then `port.list_branches`.
pub fn clone_and_list_branches(
    port: &impl ExternalSourcePort,
    repo: &ExternalRepoRef,
) -> Result<Vec<ExternalBranch>, String> {
    let target_path = format!("/tmp/{}", repo.project_id);
    let _clone_result: ExternalCloneResult = port.clone_repo(repo, &target_path)?;
    let branches: Vec<ExternalBranch> = port.list_branches(repo)?;
    Ok(branches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external_source::{
        EXTERNAL_CLONE_PROOF_SCHEMA, EXTERNAL_PR_PROOF_SCHEMA, EXTERNAL_PUSH_PROOF_SCHEMA,
        ExternalPrProof, ExternalProvider, ExternalPushProof,
    };
    use std::sync::Mutex;

    // ── Mock port ─────────────────────────────────────────────────────────────

    struct MockPort {
        clone_repo_calls: Mutex<Vec<(String, String)>>,
        create_branch_calls: Mutex<Vec<(String, String)>>,
        clone_result: Result<ExternalCloneResult, String>,
        branch_result: Result<ExternalBranch, String>,
    }

    impl MockPort {
        fn new() -> Self {
            Self {
                clone_repo_calls: Mutex::new(Vec::new()),
                create_branch_calls: Mutex::new(Vec::new()),
                clone_result: Ok(ExternalCloneResult {
                    schema: EXTERNAL_CLONE_PROOF_SCHEMA,
                    project_id: "test".to_string(),
                    clone_path: "/tmp/test".to_string(),
                    head_sha: "abc123".to_string(),
                    default_branch: "main".to_string(),
                    success: true,
                    error: None,
                }),
                branch_result: Ok(ExternalBranch {
                    name: "test-branch".to_string(),
                    head_sha: "abc123".to_string(),
                    is_default: false,
                }),
            }
        }
    }

    impl ExternalSourcePort for MockPort {
        fn clone_repo(
            &self,
            repo: &ExternalRepoRef,
            target_path: &str,
        ) -> Result<ExternalCloneResult, String> {
            self.clone_repo_calls
                .lock()
                .unwrap()
                .push((repo.remote_url.clone(), target_path.to_string()));
            self.clone_result.clone()
        }

        fn fetch(&self, _repo: &ExternalRepoRef) -> Result<Vec<ExternalBranch>, String> {
            Ok(vec![])
        }

        fn list_branches(&self, _repo: &ExternalRepoRef) -> Result<Vec<ExternalBranch>, String> {
            Ok(vec![ExternalBranch {
                name: "main".to_string(),
                head_sha: "abc123".to_string(),
                is_default: true,
            }])
        }

        fn list_collaborators(&self, _repo: &ExternalRepoRef) -> Result<Vec<String>, String> {
            Ok(vec![])
        }

        fn create_branch(
            &self,
            _repo: &ExternalRepoRef,
            branch_name: &str,
            from_sha: &str,
        ) -> Result<ExternalBranch, String> {
            self.create_branch_calls
                .lock()
                .unwrap()
                .push((branch_name.to_string(), from_sha.to_string()));
            self.branch_result.clone()
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

    // ── helpers ───────────────────────────────────────────────────────────────

    fn test_repo() -> ExternalRepoRef {
        ExternalRepoRef {
            project_id: "test".to_string(),
            remote_url: "https://example.com/test.git".to_string(),
            default_branch: "main".to_string(),
            provider: ExternalProvider::GitHub,
        }
    }

    // ── clone_and_branch tests ────────────────────────────────────────────────

    #[test]
    fn clone_and_branch_happy_path() {
        let port = MockPort::new();
        let repo = test_repo();
        let result = clone_and_branch(&port, &repo, "feature-branch", "def456");
        assert!(result.is_ok());
        let branch = result.unwrap();
        assert_eq!(branch.name, "test-branch");
        assert_eq!(branch.head_sha, "abc123");
        assert!(!branch.is_default);
    }

    #[test]
    fn clone_and_branch_clone_fails() {
        let mut port = MockPort::new();
        port.clone_result = Err("clone failed".to_string());
        let repo = test_repo();
        let result = clone_and_branch(&port, &repo, "feature-branch", "def456");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "clone failed");
    }

    #[test]
    fn clone_and_branch_create_branch_fails() {
        let mut port = MockPort::new();
        port.branch_result = Err("branch creation failed".to_string());
        let repo = test_repo();
        let result = clone_and_branch(&port, &repo, "feature-branch", "def456");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "branch creation failed");
    }

    #[test]
    fn clone_and_branch_uses_correct_target_path() {
        let port = MockPort::new();
        let repo = test_repo();
        let _ = clone_and_branch(&port, &repo, "feature-branch", "def456");
        let calls = port.clone_repo_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "https://example.com/test.git");
        assert_eq!(calls[0].1, "/tmp/test");
    }

    #[test]
    fn clone_and_branch_passes_correct_branch_params() {
        let port = MockPort::new();
        let repo = test_repo();
        let _ = clone_and_branch(&port, &repo, "feature-branch", "def456");
        let calls = port.create_branch_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "feature-branch");
        assert_eq!(calls[0].1, "def456");
    }

    #[test]
    fn clone_and_branch_clone_fails_no_branch_call() {
        let mut port = MockPort::new();
        port.clone_result = Err("clone failed".to_string());
        let repo = test_repo();
        let _ = clone_and_branch(&port, &repo, "feature-branch", "def456");
        let branch_calls = port.create_branch_calls.lock().unwrap();
        assert_eq!(branch_calls.len(), 0);
    }

    // ── clone_and_list_branches tests ─────────────────────────────────────────

    #[test]
    fn clone_and_list_branches_happy_path() {
        let port = MockPort::new();
        let repo = test_repo();
        let result = clone_and_list_branches(&port, &repo);
        assert!(result.is_ok());
        let branches = result.unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
    }

    #[test]
    fn clone_and_list_branches_clone_fails() {
        let mut port = MockPort::new();
        port.clone_result = Err("clone failed".to_string());
        let repo = test_repo();
        let result = clone_and_list_branches(&port, &repo);
        assert!(result.is_err());
    }
}
