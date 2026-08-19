use std::path::Path;
use std::process::Command;

use super::types::{SourceLayer, SourceState, SyncObservation};

pub fn observe_layer(root: &Path, layer: SourceLayer) -> SourceState {
    let remote_url = match layer {
        SourceLayer::Cortex => None,
        SourceLayer::Forgejo => read_remote_url(root, "forgejo"),
        SourceLayer::GitHub => read_remote_url(root, "origin"),
    };

    let head_sha = read_rev_parse(root, "HEAD");
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

pub fn observe_all_layers(root: &Path) -> SyncObservation {
    SyncObservation {
        schema: "cynic.host.source_sync_observation.v1",
        cortex: observe_layer(root, SourceLayer::Cortex),
        forgejo: observe_layer(root, SourceLayer::Forgejo),
        github: observe_layer(root, SourceLayer::GitHub),
        timestamp_unix_s: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}

fn read_remote_url(root: &Path, remote: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", remote])
        .current_dir(root)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn read_rev_parse(root: &Path, reference: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", reference])
        .current_dir(root)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch == "HEAD" { None } else { Some(branch) }
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
    Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .map(|o| {
            if o.status.success() {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect()
            } else {
                Vec::new()
            }
        })
        .unwrap_or_default()
}

fn read_ahead_behind(root: &Path, layer: &SourceLayer) -> (Option<u32>, Option<u32>) {
    let (upstream, downstream) = match layer {
        SourceLayer::Cortex => ("main", "forgejo/main"),
        SourceLayer::Forgejo => ("forgejo/main", "origin/main"),
        SourceLayer::GitHub => return (None, None),
    };

    let output = Command::new("git")
        .args([
            "rev-list",
            "--left-right",
            "--count",
            &format!("{}...{}", upstream, downstream),
        ])
        .current_dir(root)
        .output();

    if let Ok(o) = output {
        if o.status.success() {
            let text = String::from_utf8_lossy(&o.stdout);
            let parts: Vec<&str> = text.trim().split_whitespace().collect();
            if parts.len() == 2 {
                return (parts[0].parse().ok(), parts[1].parse().ok());
            }
        }
    }
    (None, None)
}

fn collect_blockers(
    root: &Path,
    layer: &SourceLayer,
    dirty: &[String],
    reachable: bool,
) -> Vec<String> {
    let mut blockers = Vec::new();

    if !reachable && *layer != SourceLayer::Cortex {
        blockers.push(format!("{} remote is not reachable", layer));
    }

    if layer == &SourceLayer::Cortex && !dirty.is_empty() {
        blockers.push(format!("cortex has {} dirty files", dirty.len()));
    }

    if layer == &SourceLayer::Cortex {
        let conflicts = Command::new("git")
            .args(["diff", "--name-only", "--diff-filter=U"])
            .current_dir(root)
            .output();
        if let Ok(o) = conflicts {
            if o.status.success() {
                let count = String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter(|l| !l.is_empty())
                    .count();
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
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let state = observe_layer(&root, SourceLayer::Cortex);
        assert_eq!(state.layer, SourceLayer::Cortex);
        assert!(state.head_sha.is_some());
    }

    #[test]
    fn observe_all_returns_full_observation() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let obs = observe_all_layers(&root);
        assert_eq!(obs.cortex.layer, SourceLayer::Cortex);
        assert_eq!(obs.forgejo.layer, SourceLayer::Forgejo);
        assert_eq!(obs.github.layer, SourceLayer::GitHub);
    }
}
