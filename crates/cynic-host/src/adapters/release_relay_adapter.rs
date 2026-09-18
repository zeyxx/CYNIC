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

/// Verify that the tag SHA on the remote matches the local tag SHA (read-back proof).
pub fn verify_relay_readback(root: &Path, remote: &str, tag: &str) -> RelayReadbackProof {
    let local_sha = read_local_tag_sha(root, tag);
    let remote_sha = read_remote_tag_sha(root, remote, tag);

    RelayReadbackProof {
        schema: "cynic.relay_readback_proof.v1",
        tag: tag.to_string(),
        remote: remote.to_string(),
        local_tag_sha: local_sha.clone(),
        remote_tag_sha: remote_sha.clone(),
        matches: local_sha.is_some() && local_sha == remote_sha,
    }
}

/// Read-back proof structure returned after a relay operation.
#[derive(Debug, Clone)]
pub struct RelayReadbackProof {
    pub schema: &'static str,
    pub tag: String,
    pub remote: String,
    pub local_tag_sha: Option<String>,
    pub remote_tag_sha: Option<String>,
    pub matches: bool,
}
