use std::fs;
use std::path::Path;
use std::process::Command;

use super::detect::DriftReport;

pub const REPAIR_SCHEMA: &str = "cynic.builder_repair_report.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairReport {
    pub schema: String,
    pub ref_path: String,
    pub repaired: bool,
    pub error: Option<String>,
}

/// Repair ownership of drifted refs back to `cynic:cynic`.
///
/// Uses `chown -R cynic:cynic` via sudo. Requires operator confirmation.
pub fn repair_drift(reports: &[DriftReport]) -> Result<Vec<RepairReport>, String> {
    let mut results = Vec::new();

    for report in reports {
        if !report.repairable {
            results.push(RepairReport {
                schema: REPAIR_SCHEMA.to_string(),
                ref_path: report.ref_path.clone(),
                repaired: false,
                error: Some("not repairable".to_string()),
            });
            continue;
        }

        let path = Path::new(&report.ref_path);
        let parent = path.parent().unwrap_or(path);

        let output = Command::new("sudo")
            .args(["chown", "-R", "cynic:cynic"])
            .arg(parent)
            .output()
            .map_err(|e| format!("chown failed: {e}"))?;

        let success = output.status.success();
        results.push(RepairReport {
            schema: REPAIR_SCHEMA.to_string(),
            ref_path: report.ref_path.clone(),
            repaired: success,
            error: if success {
                None
            } else {
                Some(String::from_utf8_lossy(&output.stderr).to_string())
            },
        });
    }

    Ok(results)
}

/// Check if a previous fetch failed due to permission denied.
pub fn was_fetch_permission_denied(fetch_stderr: &str) -> bool {
    fetch_stderr.contains("Permission denied") && fetch_stderr.contains(".git/refs/remotes/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_permission_denied_message() {
        let msg = "error: cannot lock ref 'refs/remotes/forgejo/agent/x': Permission denied";
        assert!(was_fetch_permission_denied(msg));
    }

    #[test]
    fn ignores_other_messages() {
        let msg = "fatal: not a git repository";
        assert!(!was_fetch_permission_denied(msg));
    }
}
