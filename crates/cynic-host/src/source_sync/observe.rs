//! Observe the 3 source layers — read-only, no mutation.
//!
//! Reads git state via shell commands (portable, no git2 dependency).
//! Reports blockers without attempting repair.

use std::path::Path;
use std::process::Command;

use super::types::{SourceLayer, SourceState, SyncObservation};

/// Observe a single source layer.
pub fn observe_layer(root: &Path, layer: SourceLayer) -> SourceState {
    let remote_url = match layer {
        SourceLayer::Cortex => None,
        SourceLayer::Forgejo => read_remote_url(root, "forgejo"),
        SourceLayer::GitHub => read_remote_url(root, "origin"),
    };

    let head_sha = read_head_sha(root);
    let branch = read_current_branch(root);
    let reachable = check_remote_reachable(root, &layer);
    let dirty_files = if layer == SourceLayer::Cortex {
        read_dirty_files(root)
    } else {
        Vec::new()
    };

    let (commits_ahead, commits_behind) = read_ahead_behind(root, &layer);

    let blockers = collect_blockers(root, &layer, &dirty_files, reachable);

    SourceState {
        layer,
        remote_url,
        head_sha,
        branch,
        commits_ahead,
        commits_behind,
        dirty_files,
        reachable,
        blockers,
    }
}

/// Observe all 3 layers.
pub fn observe_all_layers(root: &Path) -> SyncObservation {
    let cortex = observe_layer(root, SourceLayer::Cortex);
    let forgejo = observe_layer(root, SourceLayer::Forgejo);
    let github = observe_layer(root, SourceLayer::GitHub);

    SyncObservation {
        schema: "cynic.host.source_sync_observation.v1",
        cortex,
        forgejo,
        github,
        timestamp_unix_s: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}

// ── Private helpers ─────────────────────────────────────────────────

fn read_remote_url(root: &Path, remote: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", remote])
        .current_dir(root)
        .output()
        .ok()?;
    if output.status.success() {
        String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
    } else {
        None
    }
}

fn read_head_sha(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    if output.status.success() {
        String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
    } else {
        None
    }
}

fn read_current_branch(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    if output.status.success() {
        let branch = String::from_utf8(output.stdout).ok()?.trim().to_string();
        if branch == "HEAD" {
            None // detached
        } else {
            Some(branch)
        }
    } else {
        None
    }
}

fn check_remote_reachable(root: &Path, layer: &SourceLayer) -> bool {
    let remote = match layer {
        SourceLayer::Cortex => return true,
        SourceLayer::Forgejo => "forgejo",
        SourceLayer::GitHub => "origin",
    };
    Command::new("git")
        .args(["ls-remote", "--exit-code", remote])
        .current_dir(root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn read_dirty_files(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8(o.stdout)
            .unwrap_or_default()
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn read_ahead_behind(root: &Path, layer: &SourceLayer) -> (Option<u32>, Option<u32>) {
    let (upstream, downstream) = match layer {
        SourceLayer::Cortex => ("main", "forgejo/main"),
        SourceLayer::Forgejo => ("forgejo/main", "origin/main"),
        SourceLayer::GitHub => return (None, None), // terminal layer
    };

    let output = Command::new("git")
        .args(["rev-list", "--left-right", "--count", &format!("{}...{}", upstream, downstream)])
        .current_dir(root)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8(o.stdout).unwrap_or_default();
            let parts: Vec<&str> = text.trim().split_whitespace().collect();
            if parts.len() == 2 {
                let ahead = parts[0].parse().ok();
                let behind = parts[1].parse().ok();
                return (ahead, behind);
            }
        }
        _ => {}
    }
    (None, None)
}

fn collect_blockers(root: &Path, layer: &SourceLayer, dirty: &[String], reachable: bool) -> Vec<String> {
    let mut blockers = Vec::new();

    if !reachable && *layer != SourceLayer::Cortex {
        blockers.push(format!("{} remote is not reachable", layer));
    }

    if layer == &SourceLayer::Cortex && !dirty.is_empty() {
        blockers.push(format!("cortex has {} dirty files", dirty.len()));
    }

    // Check for merge conflicts markers
    if layer == &SourceLayer::Cortex {
        let output = Command::new("git")
            .args(["diff", "--name-only", "--diff-filter=U"])
            .current_dir(root)
            .output();
        if let Ok(o) = output {
            if o.status.success() {
                let conflicted = String::from_utf8(o.stdout).unwrap_or_default();
                let count = conflicted.lines().filter(|l| !l.is_empty()).count();
                if count > 0 {
                    blockers.push(format!("{} unresolved merge conflicts", count));
                }
            }
        }
    }

    blockers
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn observe_cortex_returns_state() {
        let root = PathBuf::from(".");
        let state = observe_layer(&root, SourceLayer::Cortex);
        assert_eq!(state.layer, SourceLayer::Cortex);
        assert!(state.head_sha.is_some());
    }

    #[test]
    fn observe_all_returns_full_observation() {
        let root = PathBuf::from(".");
        let obs = observe_all_layers(&root);
        assert_eq!(obs.cortex.layer, SourceLayer::Cortex);
        assert_eq!(obs.forgejo.layer, SourceLayer::Forgejo);
        assert_eq!(obs.github.layer, SourceLayer::GitHub);
    }
}
