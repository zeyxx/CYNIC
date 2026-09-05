use std::path::Path;
use std::process::Command;

use super::types::{SourceLayer, SourceState, SyncObservation};

pub fn observe_layer(root: &Path, layer: SourceLayer) -> SourceState {
    let remote_name = resolve_layer_remote(root, &layer);
    let remote_url = match layer {
        SourceLayer::Cortex => None,
        _ => remote_name
            .as_deref()
            .and_then(|remote| read_remote_url(root, remote)),
    };

    let head_sha = read_rev_parse(root, "HEAD");
    let branch = read_current_branch(root);
    let reachable = match layer {
        SourceLayer::Cortex => true,
        _ => remote_name
            .as_deref()
            .is_some_and(|remote| check_remote_reachable(root, remote)),
    };
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

fn check_remote_reachable(root: &Path, remote: &str) -> bool {
    Command::new("git")
        .args(["ls-remote", "--exit-code", remote])
        .current_dir(root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Resolve which local git remote backs a source layer by matching observed
/// remote URLs against the registry authority (`infra/repo-remotes.json`).
/// Remote names are local labels, never authority: `origin` may point at
/// Forgejo on one machine and at GitHub on another.
fn resolve_layer_remote(root: &Path, layer: &SourceLayer) -> Option<String> {
    if *layer == SourceLayer::Cortex {
        return None;
    }
    let remotes = list_git_remotes(root);
    if remotes.is_empty() {
        return None;
    }
    let sovereign = sovereign_identity(root);
    let relay = relay_identity(root);
    // Registry-backed match first.
    for (name, url) in &remotes {
        if matches_identity(url, &sovereign, &relay, layer) {
            return Some(name.clone());
        }
    }
    // Heuristic fallback when the registry is unreadable: well-known hosts.
    for (name, url) in &remotes {
        let host = url_host(url).unwrap_or_default();
        let lower = url.to_lowercase();
        let is_sovereign = host.contains("forgejo") || lower.contains("forgejo");
        let is_relay = host == "github.com" || lower.contains("github.com");
        match layer {
            SourceLayer::Forgejo if is_sovereign => return Some(name.clone()),
            SourceLayer::GitHub if is_relay => return Some(name.clone()),
            _ => {}
        }
    }
    None
}

fn matches_identity(
    url: &str,
    sovereign: &Option<RemoteIdentity>,
    relay: &Option<RemoteIdentity>,
    layer: &SourceLayer,
) -> bool {
    let identity = match layer {
        SourceLayer::Forgejo => sovereign.as_ref(),
        SourceLayer::GitHub => relay.as_ref(),
        SourceLayer::Cortex => return false,
    };
    let Some(expected) = identity else {
        return false;
    };
    let Some(host) = url_host(url) else {
        return false;
    };
    if !host.eq_ignore_ascii_case(&expected.host) {
        return false;
    }
    let lower = url.to_lowercase();
    lower.contains(&expected.owner.to_lowercase())
        && lower.contains(&expected.repository.to_lowercase())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RemoteIdentity {
    host: String,
    owner: String,
    repository: String,
}

/// Sovereign identity from the registry authority via
/// `repo_access::load_forgejo_desired_state` (host parsed from the desired
/// SSH URL, user portion is redacted and ignored).
fn sovereign_identity(root: &Path) -> Option<RemoteIdentity> {
    let desired = crate::repo_access::load_forgejo_desired_state(root).ok()?;
    let host = url_host(&desired.git_ssh_url)?;
    Some(RemoteIdentity {
        host,
        owner: desired.owner,
        repository: desired.repository,
    })
}

/// Relay identity from `infra/repo-remotes.json` (`role == "public_relay"`).
fn relay_identity(root: &Path) -> Option<RemoteIdentity> {
    let raw = std::fs::read_to_string(root.join("infra/repo-remotes.json")).ok()?;
    let document: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let remotes = document.get("remotes")?.as_array()?;
    let relay = remotes
        .iter()
        .find(|entry| entry.get("role").and_then(|role| role.as_str()) == Some("public_relay"))?;
    let url = relay
        .get("url")
        .and_then(|value| value.as_str())
        .or_else(|| {
            relay
                .get("desired_git_url")
                .and_then(|value| value.as_str())
        })?;
    let host = url_host(url)?;
    let (owner, repository) = split_owner_repo(url)?;
    Some(RemoteIdentity {
        host,
        owner,
        repository,
    })
}

fn list_git_remotes(root: &Path) -> Vec<(String, String)> {
    let output = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(root)
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let mut remotes = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        // `<name>\t<url> (fetch)` — keep fetch entries, dedupe names.
        let mut parts = line.split_whitespace();
        let (Some(name), Some(url)) = (parts.next(), parts.next()) else {
            continue;
        };
        if line.contains("(push)") {
            continue;
        }
        if remotes.iter().any(|(existing, _)| existing == name) {
            continue;
        }
        remotes.push((name.to_string(), url.to_string()));
    }
    remotes
}

/// Host portion of a git URL across `ssh://`, `https://`, and scp-like
/// `git@host:path` forms. Returns lowercase host.
fn url_host(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    if let Some(after_scheme) = url.split("://").nth(1) {
        // Strip `user@`, then read host up to ':' or '/'.
        let without_user = after_scheme.split('@').next_back().unwrap_or(after_scheme);
        let host = without_user.split([':', '/']).next().unwrap_or("").trim();
        if host.is_empty() {
            return None;
        }
        return Some(host.to_lowercase());
    }
    // Scp-like `git@github.com:owner/repo.git` or `host:path`.
    if let Some(colon) = url.find(':') {
        let left = &url[..colon];
        if left.contains('/') || left.is_empty() {
            return None;
        }
        let host = left.split('@').next_back().unwrap_or(left).trim();
        if host.is_empty() {
            return None;
        }
        return Some(host.to_lowercase());
    }
    None
}

fn split_owner_repo(url: &str) -> Option<(String, String)> {
    let path = repo_path(url)?;
    let mut segments: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    if segments.len() < 2 {
        return None;
    }
    let repository = segments
        .pop()
        .unwrap_or("")
        .trim_end_matches(".git")
        .to_string();
    let owner = segments.pop().unwrap_or("").to_string();
    if owner.is_empty() || repository.is_empty() {
        return None;
    }
    Some((owner, repository))
}

fn repo_path(url: &str) -> Option<String> {
    let url = url.trim();
    if let Some(after_scheme) = url.split("://").nth(1) {
        let without_user = after_scheme.split('@').next_back().unwrap_or(after_scheme);
        // Drop host and optional :port.
        let path = if let Some(slash) = without_user.find('/') {
            &without_user[slash + 1..]
        } else {
            return None;
        };
        return Some(path.to_string());
    }
    if let Some(colon) = url.find(':') {
        let path = url[colon + 1..].trim().trim_start_matches('/');
        if path.is_empty() {
            return None;
        }
        return Some(path.to_string());
    }
    None
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
    let sovereign = resolve_layer_remote(root, &SourceLayer::Forgejo);
    let relay = resolve_layer_remote(root, &SourceLayer::GitHub);
    let (upstream, downstream) = match layer {
        SourceLayer::Cortex => {
            let Some(sovereign) = sovereign.as_deref() else {
                return (None, None);
            };
            ("main".to_string(), format!("{sovereign}/main"))
        }
        SourceLayer::Forgejo => {
            let (Some(sovereign), Some(relay)) = (sovereign.as_deref(), relay.as_deref()) else {
                return (None, None);
            };
            (format!("{sovereign}/main"), format!("{relay}/main"))
        }
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

    #[test]
    fn url_host_covers_ssh_https_and_scp_forms() {
        assert_eq!(
            url_host("ssh://forgejo@forgejo.synapse.cynic:22/jeanterre13/CYNIC.git"),
            Some("forgejo.synapse.cynic".to_string())
        );
        assert_eq!(
            url_host("https://github.com/zeyxx/CYNIC.git"),
            Some("github.com".to_string())
        );
        assert_eq!(
            url_host("git@github.com:zeyxx/CYNIC.git"),
            Some("github.com".to_string())
        );
        assert_eq!(url_host(""), None);
    }

    #[test]
    fn remote_names_are_labels_not_authority() {
        // `origin` pointing at Forgejo must classify as sovereign, even though
        // the authority id for the relay is also named `origin`.
        let sovereign = Some(RemoteIdentity {
            host: "forgejo.synapse.cynic".to_string(),
            owner: "jeanterre13".to_string(),
            repository: "CYNIC".to_string(),
        });
        let relay = Some(RemoteIdentity {
            host: "github.com".to_string(),
            owner: "zeyxx".to_string(),
            repository: "CYNIC".to_string(),
        });
        assert!(matches_identity(
            "ssh://forgejo@forgejo.synapse.cynic:22/jeanterre13/CYNIC.git",
            &sovereign,
            &relay,
            &SourceLayer::Forgejo,
        ));
        assert!(!matches_identity(
            "ssh://forgejo@forgejo.synapse.cynic:22/jeanterre13/CYNIC.git",
            &sovereign,
            &relay,
            &SourceLayer::GitHub,
        ));
        assert!(matches_identity(
            "https://github.com/zeyxx/CYNIC.git",
            &sovereign,
            &relay,
            &SourceLayer::GitHub,
        ));
    }
}
