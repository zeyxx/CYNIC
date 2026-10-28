//! Main divergence surface: compare Forgejo (SSOT) vs GitHub (relay) main SHAs.
//!
//! Read-only by construction: runs `git ls-remote` + `git merge-base` only.
//! No push, no force-update, no mutation.

use cynic_lifecycle::{ArgvRunner, LocalArgvRunner};
use serde::Serialize;
use std::path::Path;
use std::process::ExitCode;

use crate::cli_json::emit_pretty_json;

#[derive(Debug, Serialize)]
pub(crate) struct DivergenceReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub forgejo_sha: String,
    pub github_sha: String,
    pub forgejo_url: String,
    pub github_url: String,
    pub orphan_branch_count: usize,
}

pub(crate) fn execute_divergence(root: &Path, json: bool) -> ExitCode {
    let registry_path = root.join("infra/registry.json");

    let (forgejo_url, github_url) = match resolve_remotes(&registry_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("blocked: registry error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let forgejo_sha = match ls_remote_sha(&forgejo_url, "refs/heads/main") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("blocked: forgejo ls-remote failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let github_sha = match ls_remote_sha(&github_url, "refs/heads/main") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("blocked: github ls-remote failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let status = if forgejo_sha == github_sha {
        "AGREE"
    } else {
        match ancestry_status(&forgejo_sha, &github_sha, &forgejo_url, &github_url) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("blocked: ancestry probe failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    };

    // Count orphan branches (no upstream tracking) as drift signal.
    let orphan_branch_count = {
        let output = LocalArgvRunner.run(&["git", "branch", "--format=%(refname:short) %(upstream:short)"]);
        output
            .stdout
            .lines()
            .filter(|l| {
                let parts: Vec<&str> = l.split_whitespace().collect();
                parts.len() == 1 || parts.get(1).map(|u| u.is_empty()).unwrap_or(true)
            })
            .count()
    };

    let report = DivergenceReport {
        schema: "cynic.source.divergence.v1",
        status,
        forgejo_sha: forgejo_sha.clone(),
        github_sha: github_sha.clone(),
        forgejo_url: forgejo_url.clone(),
        github_url: github_url.clone(),
        orphan_branch_count,
    };

    if json {
        emit_pretty_json(&report)
    } else {
        if status == "AGREE" {
            println!("AGREE {} (orphans={})", forgejo_sha, orphan_branch_count);
        } else {
            println!("{} gh={} fj={} (orphans={})", status, github_sha, forgejo_sha, orphan_branch_count);
        }
        ExitCode::SUCCESS
    }
}

/// Pure classification given ancestry booleans. Testable without I/O.
pub(crate) fn classify(forgejo_sha: &str, github_sha: &str, gh_ahead: bool, fj_ahead: bool) -> &'static str {
    if forgejo_sha == github_sha {
        "AGREE"
    } else if gh_ahead {
        "GH-AHEAD"
    } else if fj_ahead {
        "FORGEJO-AHEAD"
    } else {
        "DIVERGED"
    }
}

fn resolve_remotes(registry_path: &Path) -> Result<(String, String), String> {
    let raw = std::fs::read_to_string(registry_path)
        .map_err(|e| format!("registry unreadable at {}: {e}", registry_path.display()))?;
    let reg: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("registry invalid JSON: {e}"))?;

    // Forgejo SSOT.
    let ssot = cynic_vcs::find_ssot_in_registry(&reg)
        .ok_or_else(|| "vcs_provider.sovereign_ssot not found in registry".to_string())?;
    let owner = ssot["owner"].as_str().unwrap_or("jeanterre13");
    let repo = ssot["repo"].as_str().unwrap_or("CYNIC");

    // forgejo node (role forge_primary) ssh transport.
    let node = reg
        .get("nodes")
        .and_then(|n| n.as_array())
        .and_then(|nodes| {
            nodes
                .iter()
                .find(|n| n.get("role").and_then(|r| r.as_str()) == Some("forge_primary")
                    || n.get("id").and_then(|i| i.as_str()) == Some("forgejo"))
        })
        .ok_or_else(|| "forgejo node not found in registry".to_string())?;
    let ssh = node
        .pointer("/transports/ssh")
        .ok_or_else(|| "forgejo ssh transport missing".to_string())?;
    let target = ssh["target"]
        .as_str()
        .ok_or_else(|| "forgejo ssh target missing".to_string())?;
    let port = ssh["port"].as_u64().unwrap_or(22);
    let principal = ssh["principal"].as_str().unwrap_or("forgejo");
    let forgejo_url = format!("ssh://{}@{}:{}/{}.git", principal, target, port, format!("{}/{}", owner, repo));

    // GitHub relay.
    let vcs_provider = reg
        .get("nodes")
        .and_then(|n| n.as_array())
        .and_then(|nodes| {
            nodes.iter().find_map(|n| {
                n.get("source_workspace_layout")
                    .and_then(|s| s.get("vcs_provider"))
                    .or_else(|| n.get("source_lifecycle").and_then(|s| s.get("vcs_provider")))
            })
        })
        .or_else(|| reg.get("vcs_provider"));
    let relay = vcs_provider
        .and_then(|vp| vp.get("relay"))
        .ok_or_else(|| "vcs_provider.relay missing".to_string())?;
    let gh_owner = relay["owner"].as_str().unwrap_or("zeyxx");
    let gh_repo = relay["repo"].as_str().unwrap_or("CYNIC");
    let github_url = format!("https://github.com/{}/{}.git", gh_owner, gh_repo);

    Ok((forgejo_url, github_url))
}

fn ls_remote_sha(url: &str, refname: &str) -> Result<String, String> {
    let output = LocalArgvRunner.run(&["git", "ls-remote", url, refname]);
    if !output.success() {
        return Err(format!(
            "git ls-remote failed: {}",
            output.stderr.trim()
        ));
    }
    let stdout = output.stdout;
    let sha = stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| "no SHA in ls-remote output".to_string())?
        .to_string();
    if sha.is_empty() {
        return Err("empty SHA from ls-remote".to_string());
    }
    Ok(sha)
}

/// True when Forgejo main (source) and the GitHub projection main are the same
/// commit, both read live from the remotes. No local ref is consulted.
pub(crate) fn live_main_agreement(root: &Path) -> Result<bool, String> {
    let registry_path = root.join("infra/registry.json");
    let (forgejo_url, github_url) = resolve_remotes(&registry_path)?;
    let forgejo = ls_remote_sha(&forgejo_url, "refs/heads/main")?;
    let github = ls_remote_sha(&github_url, "refs/heads/main")?;
    Ok(forgejo == github)
}

fn ancestry_status(forgejo_sha: &str, github_sha: &str, forgejo_url: &str, github_url: &str) -> Result<&'static str, String> {
    let tmp = std::env::temp_dir().join(format!("cynic-divergence-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| format!("create temp dir: {e}"))?;

    let result = (|| -> Result<&'static str, String> {
        run_git(&tmp, &["init", "-q"])?;
        run_git(&tmp, &["fetch", "-q", forgejo_url, "main:fj"])?;
        run_git(&tmp, &["fetch", "-q", github_url, "main:gh"])?;
        let gh_ahead = is_ancestor(&tmp, forgejo_sha, github_sha);
        let fj_ahead = is_ancestor(&tmp, github_sha, forgejo_sha);
        Ok(classify(forgejo_sha, github_sha, gh_ahead, fj_ahead))
    })();

    let _ = std::fs::remove_dir_all(&tmp);
    result
}

fn is_ancestor(repo: &Path, ancestor: &str, descendant: &str) -> bool {
    let output = LocalArgvRunner.run(&["git", "-C", repo.to_str().unwrap_or("."), "merge-base", "--is-ancestor", ancestor, descendant]);
    output.success()
}

fn run_git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let mut argv = vec!["git", "-C", repo.to_str().unwrap_or(".")];
    argv.extend_from_slice(args);
    let output = LocalArgvRunner.run(&argv);
    if !output.success() {
        return Err(format!(
            "git {:?} failed: {}",
            args,
            output.stderr.trim()
        ));
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::classify;

    #[test]
    fn classify_agree() {
        assert_eq!(classify("abc", "abc", false, false), "AGREE");
    }

    #[test]
    fn classify_gh_ahead() {
        assert_eq!(classify("aaa", "bbb", true, false), "GH-AHEAD");
    }

    #[test]
    fn classify_forgejo_ahead() {
        assert_eq!(classify("aaa", "bbb", false, true), "FORGEJO-AHEAD");
    }

    #[test]
    fn classify_diverged() {
        assert_eq!(classify("aaa", "bbb", false, false), "DIVERGED");
    }
}
