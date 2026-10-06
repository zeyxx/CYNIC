//! Host adapter providing concrete gate proof records for release evaluation.

use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

use crate::types::GateProofRecord;

/// Collect raw host gate proofs (git cleanliness, compilation, secret scan).
///
/// Order is load-bearing: `git_clean` runs FIRST so the gate judges the
/// tree as committed, and `cargo check` runs with `--locked` so the gate
/// itself never rewrites `Cargo.lock` (a gate that dirties the tree it
/// judges always fails its own successor — observed 2026-10-06).
pub fn collect_host_gate_proofs(root: &Path) -> Vec<GateProofRecord> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut records = Vec::new();

    // 1. git clean — the tree must be pristine BEFORE any tool runs.
    let git_passed = check_git_status_clean(root);
    records.push(GateProofRecord {
        name: "git_clean".to_string(),
        passed: git_passed,
        detail: if git_passed {
            "worktree clean".to_string()
        } else {
            "worktree has uncommitted or dirty files".to_string()
        },
        observed_at_unix_s: now,
    });

    // 2. cargo check — locked: fail loudly on a stale lock file instead of
    // silently rewriting it (which would dirty the tree post-judgment).
    let (cargo_passed, cargo_detail) = run_cargo_check(root);
    records.push(GateProofRecord {
        name: "cargo_check".to_string(),
        passed: cargo_passed,
        detail: cargo_detail,
        observed_at_unix_s: now,
    });

    // 3. secret lint
    let lint_passed = run_secret_lint(root);
    records.push(GateProofRecord {
        name: "secret_lint".to_string(),
        passed: lint_passed,
        detail: if lint_passed {
            "no secrets found in source tree".to_string()
        } else {
            "secret lint detected suspicious patterns".to_string()
        },
        observed_at_unix_s: now,
    });

    // 3. git clean
    let git_passed = check_git_status_clean(root);
    records.push(GateProofRecord {
        name: "git_clean".to_string(),
        passed: git_passed,
        detail: if git_passed {
            "worktree clean".to_string()
        } else {
            "worktree has uncommitted or dirty files".to_string()
        },
        observed_at_unix_s: now,
    });

    records
}

fn run_cargo_check(root: &Path) -> (bool, String) {
    let output = Command::new("cargo")
        .args(["check", "--workspace", "--quiet", "--offline", "--locked"])
        .current_dir(root)
        .output();
    match output {
        Ok(o) if o.status.success() => (true, "workspace compiles cleanly".to_string()),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let hint: String = stderr
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("cargo check failed")
                .chars()
                .take(200)
                .collect();
            (false, format!("cargo check failed: {hint}"))
        }
        Err(e) => (false, format!("cargo check could not run: {e}")),
    }
}

fn run_secret_lint(root: &Path) -> bool {
    let observation = super::secret_lint::scan_for_secrets(root);
    !observation.has_findings()
}

fn check_git_status_clean(root: &Path) -> bool {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output();
    match output {
        Ok(o) => o.status.success() && o.stdout.is_empty(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .expect("git runs")
                .status;
            assert!(status.success(), "git {args:?} failed");
        };
        run(&["init"]);
        run(&["config", "user.email", "gate@test.local"]);
        run(&["config", "user.name", "gate-test"]);
        std::fs::write(dir.path().join("file.txt"), "committed").expect("write");
        run(&["add", "."]);
        run(&["commit", "-qm", "init"]);
        dir
    }

    #[test]
    fn clean_tree_passes_git_clean() {
        let dir = init_repo();
        assert!(check_git_status_clean(dir.path()));
    }

    #[test]
    fn dirty_tree_fails_git_clean() {
        let dir = init_repo();
        std::fs::write(dir.path().join("file.txt"), "dirty").expect("dirty write");
        assert!(!check_git_status_clean(dir.path()));
    }

    #[test]
    fn untracked_file_fails_git_clean() {
        let dir = init_repo();
        std::fs::write(dir.path().join("untracked.txt"), "new").expect("write");
        assert!(!check_git_status_clean(dir.path()));
    }
}
