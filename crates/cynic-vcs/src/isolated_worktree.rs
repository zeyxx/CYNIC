//! Isolated worktree allocation and lifecycle specification.
//!
//! Enforces Foundation §481 and `contracts/source-lifecycle.json`.
//! Pure zero-I/O validation and path derivation for concurrent agent worktrees.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

pub const ISOLATED_WORKTREE_ALLOCATION_SCHEMA: &str = "cynic.vcs.isolated_worktree.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeAllocationRequest {
    pub branch: String,
    pub base_ref: Option<String>,
    pub target_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeAllocationPlan {
    pub schema: &'static str,
    pub branch: String,
    pub base_ref: String,
    pub target_path: PathBuf,
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeAllocationReceipt {
    pub status: &'static str,
    pub branch: String,
    pub base_ref: String,
    pub target_path: PathBuf,
    pub schema: &'static str,
}

/// Execute a planned worktree allocation through an injectable ArgvRunner.
pub fn execute_worktree_allocation<R: cynic_protocol::argv::ArgvRunner>(
    plan: &WorktreeAllocationPlan,
    runner: &R,
) -> Result<WorktreeAllocationReceipt, String> {
    if let Some(parent) = plan.target_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let argv_slices: Vec<&str> = plan.argv.iter().map(|s| s.as_str()).collect();
    let output = runner.run(&argv_slices);

    if !output.success() {
        return Err(output.summary());
    }

    Ok(WorktreeAllocationReceipt {
        status: "allocated",
        branch: plan.branch.clone(),
        base_ref: plan.base_ref.clone(),
        target_path: plan.target_path.clone(),
        schema: ISOLATED_WORKTREE_ALLOCATION_SCHEMA,
    })
}

/// Slugify a branch name safely for directory allocation.
pub fn branch_to_directory_slug(branch: &str) -> String {
    branch
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

/// Derive the canonical target path for an isolated agent worktree.
pub fn derive_isolated_worktree_path(root_worktree_dir: &Path, branch: &str) -> PathBuf {
    let slug = branch_to_directory_slug(branch);
    root_worktree_dir.join(slug)
}

/// Validate that a branch name is admissible for an isolated agent worktree.
pub fn validate_worktree_branch_admission(branch: &str) -> Result<(), &'static str> {
    let trimmed = branch.trim();
    if trimmed.is_empty() {
        return Err("branch_name_empty");
    }
    if trimmed == "main" || trimmed == "master" || trimmed == "origin/main" {
        return Err("cannot_allocate_worktree_for_root_main_branch");
    }
    if trimmed.starts_with('-') || trimmed.contains("..") || trimmed.contains('@') {
        return Err("invalid_git_branch_characters");
    }
    Ok(())
}

/// Plan the git worktree allocation without executing I/O.
pub fn plan_worktree_allocation(
    repo_root: &Path,
    default_worktree_dir: &Path,
    request: &WorktreeAllocationRequest,
) -> Result<WorktreeAllocationPlan, &'static str> {
    validate_worktree_branch_admission(&request.branch)?;

    let base_ref = request
        .base_ref
        .clone()
        .unwrap_or_else(|| "origin/main".to_string());

    let target_path = request
        .target_path
        .clone()
        .unwrap_or_else(|| derive_isolated_worktree_path(default_worktree_dir, &request.branch));

    let repo_str = repo_root.to_string_lossy().to_string();
    let target_str = target_path.to_string_lossy().to_string();

    let argv = vec![
        "git".to_string(),
        "-C".to_string(),
        repo_str,
        "worktree".to_string(),
        "add".to_string(),
        // -b, not -B: -B silently resets an existing branch to base_ref (data loss).
        "-b".to_string(),
        request.branch.clone(),
        target_str,
        base_ref.clone(),
    ];

    Ok(WorktreeAllocationPlan {
        schema: ISOLATED_WORKTREE_ALLOCATION_SCHEMA,
        branch: request.branch.clone(),
        base_ref,
        target_path,
        argv,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_branch_name_rules() {
        assert!(validate_worktree_branch_admission("agent/hermes/task-1").is_ok());
        assert_eq!(
            validate_worktree_branch_admission("main"),
            Err("cannot_allocate_worktree_for_root_main_branch")
        );
        assert_eq!(
            validate_worktree_branch_admission(""),
            Err("branch_name_empty")
        );
        assert_eq!(
            validate_worktree_branch_admission("bad..branch"),
            Err("invalid_git_branch_characters")
        );
    }

    #[test]
    fn derives_safe_slug_path() {
        let base_dir = Path::new("/var/lib/cynic/worktrees");
        let path = derive_isolated_worktree_path(base_dir, "agent/hermes/refactor-1");
        assert_eq!(
            path,
            PathBuf::from("/var/lib/cynic/worktrees/agent_hermes_refactor-1")
        );
    }

    #[test]
    fn executes_worktree_allocation_with_stubbed_runner() {
        use cynic_process::MapArgvRunner;
        use cynic_protocol::argv::ArgvOutput;

        let repo = Path::new("/home/user/CYNIC");
        let ws_root = Path::new("/tmp/cynic-vcs-wt-test");
        let target = PathBuf::from("/tmp/cynic-vcs-wt-test/agent_worker_poh-fix");
        let req = WorktreeAllocationRequest {
            branch: "agent/worker/poh-fix".to_string(),
            base_ref: None,
            target_path: Some(target.clone()),
        };

        let plan = plan_worktree_allocation(repo, ws_root, &req).unwrap();
        let runner = MapArgvRunner {
            responses: vec![(
                vec![],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: String::new(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };

        let receipt = execute_worktree_allocation(&plan, &runner).unwrap();
        assert_eq!(receipt.status, "allocated");
        assert_eq!(receipt.branch, plan.branch);
        assert_eq!(receipt.base_ref, plan.base_ref);
        assert_eq!(receipt.target_path, plan.target_path);
    }
    #[test]
    fn plans_allocation_purely() {
        let repo = Path::new("/home/user/CYNIC");
        let ws_root = Path::new("/var/lib/cynic/worktrees");
        let req = WorktreeAllocationRequest {
            branch: "agent/worker/poh-fix".to_string(),
            base_ref: None,
            target_path: None,
        };

        let plan = plan_worktree_allocation(repo, ws_root, &req).unwrap();
        assert_eq!(plan.base_ref, "origin/main");
        assert_eq!(
            plan.target_path,
            PathBuf::from("/var/lib/cynic/worktrees/agent_worker_poh-fix")
        );
        assert_eq!(
            plan.argv,
            vec![
                "git",
                "-C",
                "/home/user/CYNIC",
                "worktree",
                "add",
                "-b",
                "agent/worker/poh-fix",
                "/var/lib/cynic/worktrees/agent_worker_poh-fix",
                "origin/main"
            ]
        );
    }
}
