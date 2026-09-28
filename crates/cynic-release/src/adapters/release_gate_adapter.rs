//! Host adapter providing concrete gate proof records for release evaluation.

use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

use crate::types::GateProofRecord;

/// Collect raw host gate proofs (compilation, secret scan, git cleanliness).
pub fn collect_host_gate_proofs(root: &Path) -> Vec<GateProofRecord> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut records = Vec::new();

    // 1. cargo check
    let cargo_passed = run_cargo_check(root);
    records.push(GateProofRecord {
        name: "cargo_check".to_string(),
        passed: cargo_passed,
        detail: if cargo_passed {
            "workspace compiles cleanly".to_string()
        } else {
            "cargo check failed".to_string()
        },
        observed_at_unix_s: now,
    });

    // 2. secret lint
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

fn run_cargo_check(root: &Path) -> bool {
    let output = Command::new("cargo")
        .args(["check", "--workspace", "--quiet", "--offline"])
        .current_dir(root)
        .output();
    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
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
