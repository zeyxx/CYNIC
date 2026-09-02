//! Observe a patch for judging — diff vs origin/main + topology context.

use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::judge_patch::{PatchObservation, JUDGE_PATCH_SCHEMA};
use sha2::{Digest, Sha256};

fn run_git(root: &Path, args: &[&str]) -> Option<String> {
    Command::new("git").args(args).current_dir(root).output().ok().and_then(|o| {
        if o.status.success() { Some(String::from_utf8_lossy(&o.stdout).to_string()) } else { None }
    })
}

fn run_git_bytes(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    Command::new("git").args(args).current_dir(root).output().ok().and_then(|o| {
        if o.status.success() { Some(o.stdout) } else { None }
    })
}

pub fn observe_patch(root: &Path, branch: &str) -> PatchObservation {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let base = run_git(root, &["rev-parse", "origin/main"]).unwrap_or_default().trim().to_string();
    let diff_bytes = run_git_bytes(root, &["diff", &format!("origin/main...{branch}")]).unwrap_or_default();
    let hash = hex::encode(Sha256::digest(&diff_bytes));
    let staged = run_git(root, &["diff", "--cached", "--name-only"])
        .map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default();
    // ahead/behind from topology if available, else try git rev-list
    let ahead = run_git(root, &["rev-list", "--count", &format!("origin/main..{branch}")])
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let behind = run_git(root, &["rev-list", "--count", &format!("{branch}..origin/main")])
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0);

    PatchObservation {
        schema: JUDGE_PATCH_SCHEMA.to_string(),
        branch: branch.to_string(),
        base_commit: base,
        diff_hash: hash,
        diff_bytes_len: diff_bytes.len(),
        ahead,
        behind,
        staged_files: staged,
        observed_at_unix_s: now,
    }
}
