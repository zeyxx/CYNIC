use sha2::{Digest, Sha256};
use std::fs;

use crate::filesystem_observation::{path_executable, path_exists, path_mode_equals};
use crate::install_contract::MODE_0440;

use super::types::{HostExecBridgeChecks, HostExecBridgeStatus};

/// Observe the privilege membrane on a node: the single executor binary
/// plus the single sudoers file that admits it. No helper binary exists
/// in this design; the executor is basket-managed (present + executable
/// is our whole contract on it), while the membrane file is installed by
/// the bridge (mode and content are verified).
pub(super) fn bridge_membrane_status(
    executor: &str,
    membrane_target: &str,
    subject: &str,
) -> HostExecBridgeStatus {
    let executor_sha256 = file_sha256(executor);
    let membrane_sha256 = file_sha256(membrane_target);
    let membrane_content = fs::read_to_string(membrane_target).unwrap_or_default();
    let admitted_prefix = format!("{subject} ALL=(root) NOPASSWD: {executor} privilege exec");
    let admits_executor = membrane_content.contains(&admitted_prefix)
        && membrane_content
            .lines()
            .filter(|line| {
                let line = line.trim();
                !line.is_empty() && !line.starts_with('#')
            })
            .all(|line| line.starts_with(&admitted_prefix));
    let checks = HostExecBridgeChecks {
        executor_exists: path_exists(executor),
        executor_executable: path_executable(executor),
        membrane_exists: path_exists(membrane_target),
        membrane_mode_0440: path_mode_equals(membrane_target, MODE_0440.unix),
        membrane_admits_executor: admits_executor,
        membrane_no_unbounded_executor: !membrane_content.lines().any(|line| {
            line.trim() == format!("{subject} ALL=(root) NOPASSWD: {executor}").as_str()
        }),
        membrane_no_shell: !membrane_content.contains("/bin/sh")
            && !membrane_content.contains("bash"),
        membrane_no_raw_sudo: !membrane_content.contains("systemctl")
            && !membrane_content.contains("apt-get"),
    };

    let mut missing = Vec::new();
    if !checks.executor_exists {
        missing.push("executor_exists".to_string());
    }
    if !checks.executor_executable {
        missing.push("executor_executable".to_string());
    }
    if !checks.membrane_exists {
        missing.push("membrane_exists".to_string());
    }
    if !checks.membrane_mode_0440 {
        missing.push("membrane_mode_0440".to_string());
    }
    if !checks.membrane_admits_executor {
        missing.push("membrane_admits_executor".to_string());
    }
    if !checks.membrane_no_unbounded_executor {
        missing.push("membrane_no_unbounded_executor".to_string());
    }
    if !checks.membrane_no_shell {
        missing.push("membrane_no_shell".to_string());
    }
    if !checks.membrane_no_raw_sudo {
        missing.push("membrane_no_raw_sudo".to_string());
    }

    HostExecBridgeStatus {
        schema: "cynic.host_exec_bridge.status.v1".to_string(),
        status: if missing.is_empty() {
            "active".to_string()
        } else {
            "missing_activation".to_string()
        },
        executor: executor.to_string(),
        membrane_target: membrane_target.to_string(),
        admitted_prefix,
        executor_sha256,
        membrane_sha256,
        checks,
        missing,
    }
}

fn file_sha256(path: &str) -> Option<String> {
    fs::read(path)
        .ok()
        .map(|bytes| format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
}
