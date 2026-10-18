//! CLI-facing observation for worktree convergence and publication planning.
//!
//! These functions encapsulate the read-side orchestration the CLI driving
//! adapter needs: canonicalize roots, resolve the workspace store, run the
//! inventory observation, and project entries into the pure snapshot shape
//! the convergence engine consumes. The CLI keeps only rendering and the
//! apply decision — no path arithmetic, no git process spawns.

use std::path::{Path, PathBuf};

use crate::git_topology_observer::{count_dirty_files, observe_head_sha};
use crate::runner::VcsRunner;
use crate::source_worktree::{ObservedWorktreeSnapshot, reconcile_worktree_convergence_pure};
use crate::observe_source_worktree_inventory;

/// Observe the worktree inventory and project it into convergence snapshots.
///
/// Returns the main HEAD SHA (for the plan's default branch) and one
/// snapshot per inventory entry. The active root is recognised by path
/// basis, relative path, or candidate id — never by a hardcoded name.
///
/// The workspace store falls back to `$HOME/.workspace` when the declared
/// store does not exist, mirroring the historical CLI behaviour so an
/// operator with the legacy layout keeps working.
pub fn observe_worktree_convergence_snapshots(
    root: &Path,
    workspace_store: &Path,
    runner: &impl VcsRunner,
    now_ms: u64,
) -> (String, Vec<ObservedWorktreeSnapshot>) {
    let root_canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let ws_store_canonical = if workspace_store.exists() {
        workspace_store.to_path_buf()
    } else {
        let home_ws = std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".workspace"))
            .unwrap_or_default();
        if home_ws.exists() {
            home_ws
        } else {
            workspace_store.to_path_buf()
        }
    };

    let inventory = observe_source_worktree_inventory(
        root,
        &ws_store_canonical,
        "cynic",
        "local",
        runner,
        now_ms,
    );
    let head_sha = observe_head_sha(&root_canonical).unwrap_or_else(|| "HEAD".to_string());

    let mut snapshots = Vec::new();
    for entry in &inventory.entries {
        let is_active_root = entry.path_basis
            == crate::source_worktree_inventory::SourceWorktreeInventoryPathBasis::MainRepository
            || entry.relative_path.as_deref() == Some(".")
            || entry.relative_path.as_deref() == Some("")
            || entry.candidate_id == "cynic";

        let path = if is_active_root {
            root_canonical.clone()
        } else {
            entry
                .relative_path
                .as_deref()
                .map(|r| ws_store_canonical.join(r))
                .unwrap_or_else(|| PathBuf::from(&entry.candidate_id))
        };

        let branch = entry
            .branch
            .as_deref()
            .map(|b| b.strip_prefix("refs/heads/").unwrap_or(b).to_string());

        let dirty_file_count = if path.exists() {
            count_dirty_files(&path)
        } else {
            0
        };

        let created_at_ms = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64);

        snapshots.push(ObservedWorktreeSnapshot {
            path,
            branch,
            head_sha: entry.head.clone().unwrap_or_default(),
            is_active_root,
            dirty_file_count,
            git_prunable: entry.git_prunable == Some(true)
                || entry.state == crate::source_worktree_inventory::SourceWorktreeInventoryEntryState::Malformed,
            prunable_reason: entry.prunable_reason.clone(),
            created_at_ms,
        });
    }

    (head_sha, snapshots)
}

/// Build the convergence plan from observed snapshots.
///
/// Thin wrapper over the pure engine so the CLI does not re-derive the
/// default branch or re-import the engine's types.
pub fn plan_worktree_convergence(
    root_canonical: PathBuf,
    default_branch: String,
    head_sha: String,
    snapshots: &[ObservedWorktreeSnapshot],
) -> crate::source_worktree::WorktreeConvergencePlan {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    reconcile_worktree_convergence_pure(root_canonical, &default_branch, &head_sha, snapshots, &[], now_ms)
}

/// Observe the inputs `plan_publication` needs: the tracked file list and
/// the bytes of every path the contract publishes.
///
/// `git ls-files -z` is used unquoted so non-ASCII paths survive. The
/// bytes are read lossily — the plan's job is to surface literals, not to
/// preserve bytes. Non-UTF-8 files are read as lossy strings.
pub fn observe_publication_inputs(
    root: &Path,
    contract: &crate::publication::PublicationContract,
    runner: &impl VcsRunner,
) -> Result<(Vec<String>, Vec<(String, String)>), String> {
    let root_str = root.to_string_lossy().to_string();
    let argv = ["git", "-C", &root_str, "ls-files", "-z"];
    let output = runner.run(&argv);
    if output.exit_code != Some(0) || output.spawn_error.is_some() {
        return Err("git ls-files exited non-zero".to_string());
    }
    let tracked: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();

    let mut files: Vec<(String, String)> = Vec::new();
    for path in &tracked {
        if contract
            .rule_for(path)
            .is_some_and(|r| r.status.is_published())
        {
            if let Ok(bytes) = std::fs::read(root.join(path)) {
                files.push((path.clone(), String::from_utf8_lossy(&bytes).to_string()));
            }
        }
    }
    Ok((tracked, files))
}
