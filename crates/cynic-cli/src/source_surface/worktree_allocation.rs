//! Helper to allocate an isolated worktree for an agent.

use std::path::Path;
use std::process::ExitCode;
use crate::cli_json::emit_pretty_json;

pub(crate) fn execute_allocate_worktree(
    root: &Path,
    workspace_store: &Path,
    branch: &str,
    base: Option<&str>,
    explicit_path: Option<&Path>,
    json: bool,
) -> ExitCode {
    let req = cynic_vcs::isolated_worktree::WorktreeAllocationRequest {
        branch: branch.to_string(),
        base_ref: base.map(|s| s.to_string()),
        target_path: explicit_path.map(|p| p.to_path_buf()),
    };

    let plan = match cynic_vcs::isolated_worktree::plan_worktree_allocation(root, workspace_store, &req) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("blocked: worktree allocation plan failed: {err}");
            return ExitCode::FAILURE;
        }
    };

    // Ensure parent directory exists
    if let Some(parent) = plan.target_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Execute through cynic-host's runner
    let runner = cynic_host::adapters::vcs_runner::LocalVcsRunner;
    let argv_slices: Vec<&str> = plan.argv.iter().map(|s| s.as_str()).collect();
    let output = cynic_vcs::ArgvRunner::run(&runner, &argv_slices);

    if !output.success() {
        eprintln!("blocked: git worktree add failed: {}", output.summary());
        return ExitCode::FAILURE;
    }

    if json {
        emit_pretty_json(&serde_json::json!({
            "schema": "cynic.vcs.isolated_worktree_allocated.v1",
            "status": "allocated",
            "branch": plan.branch,
            "base_ref": plan.base_ref,
            "target_path": plan.target_path,
        }))
    } else {
        println!("=== Isolated Worktree Allocated ===");
        println!("Branch:      {}", plan.branch);
        println!("Base Ref:    {}", plan.base_ref);
        println!("Target Path: {}", plan.target_path.display());
        println!("\nWork in isolation with: cd {}", plan.target_path.display());
        ExitCode::SUCCESS
    }
}
