//! Host adapter for GitHub relay actuation and read-back verification.

use std::path::Path;
use std::process::Command;

/// Read the SHA a tag points to on a remote, if it exists.
pub fn read_remote_tag_sha(root: &Path, remote: &str, tag: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["ls-remote", "--tags", remote, tag])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().next())
        .map(|s| s.trim().to_string())
}

/// Read the local SHA a tag points to, if it exists.
pub fn read_local_tag_sha(root: &Path, tag: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", &format!("refs/tags/{tag}")])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Read the SHA of a branch on a remote.
pub fn read_remote_branch_sha(root: &Path, remote: &str, branch: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["ls-remote", "--heads", remote, branch])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().next())
        .map(|s| s.trim().to_string())
}

/// Read the local SHA a branch points to, if it exists.
pub fn read_local_branch_sha(root: &Path, branch: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", &format!("refs/heads/{branch}")])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Push a tag to a remote.
pub fn push_tag_to_remote(root: &Path, remote: &str, tag: &str) -> bool {
    let output = Command::new("git")
        .args(["push", remote, &format!("refs/tags/{tag}")])
        .current_dir(root)
        .output();
    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

/// Force-push a tag to a remote (used for relay projection when divergent).
pub fn force_push_tag_to_remote(root: &Path, remote: &str, tag: &str) -> bool {
    let output = Command::new("git")
        .args(["push", "--force", remote, &format!("refs/tags/{tag}")])
        .current_dir(root)
        .output();
    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

/// Push a branch to a remote.
pub fn push_branch_to_remote(root: &Path, remote: &str, branch: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["push", remote, branch])
        .current_dir(root)
        .output()
        .map_err(|e| format!("git push {} {} failed: {}", remote, branch, e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git push {} {}: {}",
            remote,
            branch,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Force-push a branch to a remote.
pub fn force_push_branch_to_remote(root: &Path, remote: &str, branch: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["push", "--force", remote, branch])
        .current_dir(root)
        .output()
        .map_err(|e| format!("git push --force {} {} failed: {}", remote, branch, e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git push --force {} {}: {}",
            remote,
            branch,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Create a pull request using the GitHub CLI.
pub fn create_pull_request(
    root: &Path,
    title: &str,
    body: &str,
    base: &str,
    head: &str,
) -> Result<String, String> {
    let output = Command::new("gh")
        .args([
            "pr", "create", "--title", title, "--body", body, "--base", base, "--head", head,
        ])
        .current_dir(root)
        .output()
        .map_err(|e| format!("gh pr create failed: {}", e))?;
    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(url)
    } else {
        Err(format!(
            "gh pr create: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Verify that the tag SHA on the remote matches the local tag SHA (read-back proof).
pub fn verify_relay_readback(root: &Path, remote: &str, tag: &str) -> RelayReadbackProof {
    let local_sha = read_local_tag_sha(root, tag);
    let remote_sha = read_remote_tag_sha(root, remote, tag);
    let matches = local_sha.is_some() && local_sha == remote_sha;

    RelayReadbackProof {
        schema: "cynic.relay_readback_proof.v1",
        target: tag.to_string(),
        remote: remote.to_string(),
        local_sha,
        remote_sha,
        matches,
    }
}

/// Verify branch read-back proof.
pub fn verify_branch_readback(root: &Path, remote: &str, branch: &str) -> RelayReadbackProof {
    let local_sha = read_local_branch_sha(root, branch);
    let remote_sha = read_remote_branch_sha(root, remote, branch);
    let matches = local_sha.is_some() && local_sha == remote_sha;

    RelayReadbackProof {
        schema: "cynic.relay_readback_proof.v1",
        target: branch.to_string(),
        remote: remote.to_string(),
        local_sha,
        remote_sha,
        matches,
    }
}

/// Read-back proof structure returned after a relay operation.
#[derive(Debug, Clone)]
pub struct RelayReadbackProof {
    pub schema: &'static str,
    pub target: String,
    pub remote: String,
    pub local_sha: Option<String>,
    pub remote_sha: Option<String>,
    pub matches: bool,
}
