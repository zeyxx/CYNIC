//! `GET /git/state` — Multi-cortex SSOT for git coordination.
//!
//! Returns live git state (HEAD, branches, dirty tree, divergence) annotated
//! with cortex ownership via the coord system. Enables kernel-level git-state
//! observation for multi-cortex fork detection.

use axum::extract::State;
use axum::http::StatusCode;
use serde_json::{Value, json};
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};

// ── Git subprocess helpers ──

async fn run_git_cmd(project_root: &str, args: &[&str]) -> Result<String, String> {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::process::Command::new("git")
            .args(args)
            .current_dir(project_root)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) if output.status.success() => {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(if stderr.is_empty() {
                "git command failed".to_string()
            } else {
                stderr
            })
        }
        Ok(Err(e)) => Err(format!("Failed to spawn git: {e}")),
        Err(_) => Err("git command timed out (10s)".to_string()),
    }
}

// ── Handler ──

pub async fn git_state_handler(
    State(state): State<Arc<AppState>>,
) -> Result<axum::Json<Value>, (StatusCode, axum::Json<ErrorResponse>)> {
    let project_root = &state.project_root;

    // Fetch HEAD info
    let head_hash = run_git_cmd(project_root, &["rev-parse", "--short", "HEAD"])
        .await
        .unwrap_or_else(|e| format!("error: {e}"));
    let head_full_hash = run_git_cmd(project_root, &["rev-parse", "HEAD"])
        .await
        .unwrap_or_else(|e| format!("error: {e}"));
    let head_timestamp = run_git_cmd(project_root, &["log", "-1", "--format=%ci", "HEAD"])
        .await
        .unwrap_or_else(|e| format!("error: {e}"));
    let head_message = run_git_cmd(project_root, &["log", "-1", "--format=%s", "HEAD"])
        .await
        .unwrap_or_else(|e| format!("error: {e}"));
    let current_branch = run_git_cmd(project_root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .unwrap_or_else(|e| format!("error: {e}"));

    // Fetch branch list (name|hash|upstream)
    let branches_raw = run_git_cmd(
        project_root,
        &[
            "branch",
            "-vv",
            "--format=%(refname:short)|%(objectname:short)|%(upstream:short)",
        ],
    )
    .await
    .unwrap_or_default();

    // Parse branches and compute divergence
    let mut branches = Vec::new();
    for line in branches_raw.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 2 {
            continue;
        }
        let branch_name = parts[0].trim();
        let branch_hash = parts[1].trim();
        let upstream = if parts.len() > 2 && !parts[2].is_empty() {
            parts[2].to_string()
        } else {
            String::new()
        };

        // Compute ahead/behind if upstream exists
        let (ahead, behind) = if !upstream.is_empty() {
            match run_git_cmd(
                project_root,
                &[
                    "rev-list",
                    "--left-right",
                    "--count",
                    &format!("{}...{}", branch_name, upstream),
                ],
            )
            .await
            {
                Ok(counts_str) => {
                    let parts: Vec<&str> = counts_str.split_whitespace().collect();
                    if parts.len() == 2 {
                        (
                            parts[0].parse::<i32>().unwrap_or(0),
                            parts[1].parse::<i32>().unwrap_or(0),
                        )
                    } else {
                        (0, 0)
                    }
                }
                Err(_) => (0, 0),
            }
        } else {
            (0, 0)
        };

        branches.push(json!({
            "name": branch_name,
            "head": branch_hash,
            "ahead": ahead,
            "behind": behind,
            "upstream": upstream,
        }));
    }

    // Fetch dirty tree (modified + untracked)
    let dirty_raw = run_git_cmd(project_root, &["status", "--porcelain"])
        .await
        .unwrap_or_default();

    let mut modified = Vec::new();
    let mut untracked = Vec::new();
    for line in dirty_raw.lines() {
        if line.len() < 3 {
            continue;
        }
        let status = &line[..2];
        let path = &line[3..];
        match status {
            "??" => untracked.push(path.to_string()),
            _ if status.starts_with('M') || status.ends_with('M') => {
                modified.push(path.to_string())
            }
            _ => {}
        }
    }

    // Compute divergence for current branch and main
    let (main_ahead, main_behind) = match run_git_cmd(
        project_root,
        &["rev-list", "--left-right", "--count", "main...origin/main"],
    )
    .await
    {
        Ok(counts_str) => {
            let parts: Vec<&str> = counts_str.split_whitespace().collect();
            if parts.len() == 2 {
                (
                    parts[0].parse::<i32>().unwrap_or(0),
                    parts[1].parse::<i32>().unwrap_or(0),
                )
            } else {
                (0, 0)
            }
        }
        Err(_) => (0, 0),
    };

    let (current_ahead, current_behind) = match run_git_cmd(
        project_root,
        &[
            "rev-list",
            "--left-right",
            "--count",
            &format!("{}...origin/{}", current_branch, current_branch),
        ],
    )
    .await
    {
        Ok(counts_str) => {
            let parts: Vec<&str> = counts_str.split_whitespace().collect();
            if parts.len() == 2 {
                (
                    parts[0].parse::<i32>().unwrap_or(0),
                    parts[1].parse::<i32>().unwrap_or(0),
                )
            } else {
                (0, 0)
            }
        }
        Err(_) => (0, 0),
    };

    // Fetch coord claims and cross-ref branches
    let coord_snapshot = match state.coord.who(None).await {
        Ok(snapshot) => snapshot,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(ErrorResponse {
                    error: "Failed to query coordination state".into(),
                }),
            ));
        }
    };

    // Annotate branches with owning cortex (from file claims on branch-related files)
    let mut annotated_branches = Vec::new();
    for mut branch_obj in branches {
        let branch_name = branch_obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Look for a claim that matches this branch (heuristic: file path contains branch name)
        let cortex = coord_snapshot
            .claims
            .iter()
            .find(|claim| claim.target.contains(branch_name) || claim.target.contains("HEAD"))
            .map(|claim| claim.agent_id.clone())
            .unwrap_or_else(|| "unclaimed".to_string());

        if let Some(obj) = branch_obj.as_object_mut() {
            obj.insert("cortex".to_string(), Value::String(cortex));
        }
        annotated_branches.push(branch_obj);
    }

    Ok(axum::Json(json!({
        "head": {
            "hash": head_hash,
            "full_hash": head_full_hash,
            "timestamp": head_timestamp,
            "message": head_message,
            "branch": current_branch,
        },
        "branches": annotated_branches,
        "dirty": {
            "modified": modified,
            "untracked": untracked,
        },
        "divergence": {
            "main": {
                "ahead": main_ahead,
                "behind": main_behind,
            },
            "current_branch": {
                "ahead": current_ahead,
                "behind": current_behind,
            },
        },
        "coord_snapshot": {
            "active_agents": coord_snapshot.agents.len(),
            "active_claims": coord_snapshot.claims.len(),
        },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}
