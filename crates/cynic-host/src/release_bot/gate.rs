use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub schema: &'static str,
    pub passed: bool,
    pub checks: Vec<GateCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateCheck {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

impl GateResult {
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }
}

pub fn run_gate(root: &Path) -> GateResult {
    let mut checks = Vec::new();

    // Check 1: cargo check
    let cargo_passed = run_cargo_check(root);
    checks.push(GateCheck {
        name: "cargo_check",
        passed: cargo_passed,
        detail: if cargo_passed {
            "workspace compiles".to_string()
        } else {
            "cargo check failed".to_string()
        },
    });

    // Check 2: secret lint
    let lint_passed = run_secret_lint(root);
    checks.push(GateCheck {
        name: "secret_lint",
        passed: lint_passed,
        detail: if lint_passed {
            "no secrets found".to_string()
        } else {
            "secret lint found suspicious lines".to_string()
        },
    });

    // Check 3: git status clean
    let git_passed = check_git_status_clean(root);
    checks.push(GateCheck {
        name: "git_clean",
        passed: git_passed,
        detail: if git_passed {
            "worktree clean".to_string()
        } else {
            "worktree has dirty files".to_string()
        },
    });

    GateResult {
        schema: "cynic.host.release_gate.v1",
        passed: checks.iter().all(|c| c.passed),
        checks,
    }
}

fn run_cargo_check(root: &Path) -> bool {
    let output = std::process::Command::new("cargo")
        .args(["check", "--workspace", "--quiet"])
        .current_dir(root)
        .output();
    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

fn run_secret_lint(root: &Path) -> bool {
    let observation = crate::secret_lint::scan_for_secrets(root);
    !observation.has_findings()
}

fn check_git_status_clean(root: &Path) -> bool {
    let output = std::process::Command::new("git")
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

    #[test]
    fn gate_all_passed_true_when_all_pass() {
        let gate = GateResult {
            schema: "test",
            passed: true,
            checks: vec![
                GateCheck { name: "a", passed: true, detail: "".to_string() },
                GateCheck { name: "b", passed: true, detail: "".to_string() },
            ],
        };
        assert!(gate.all_passed());
    }

    #[test]
    fn gate_all_passed_false_when_one_fails() {
        let gate = GateResult {
            schema: "test",
            passed: false,
            checks: vec![
                GateCheck { name: "a", passed: true, detail: "".to_string() },
                GateCheck { name: "b", passed: false, detail: "".to_string() },
            ],
        };
        assert!(!gate.all_passed());
    }
}
