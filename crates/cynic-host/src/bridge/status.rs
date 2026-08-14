use sha2::{Digest, Sha256};
use std::fs;

use crate::bounded_helper::BoundedHelperProjection;
use crate::filesystem_observation::{path_executable, path_exists, path_mode_equals};
use crate::install_contract::{MODE_0440, MODE_0755};

use super::types::{HostExecBridgeChecks, HostExecBridgeStatus};

pub(super) fn bridge_status_for_projection(
    projection: &BoundedHelperProjection,
) -> HostExecBridgeStatus {
    let helper_path = projection.helper_path.clone();
    let sudoers_path = projection.sudoers_path.clone();
    let helper_sha256 = file_sha256(&helper_path);
    let sudoers_sha256 = file_sha256(&sudoers_path);
    let sudoers_content = fs::read_to_string(&sudoers_path).unwrap_or_default();
    let checks = HostExecBridgeChecks {
        helper_exists: path_exists(&helper_path),
        helper_mode_0755: path_mode_equals(&helper_path, MODE_0755.unix),
        helper_executable: path_executable(&helper_path),
        sudoers_exists: path_exists(&sudoers_path),
        sudoers_mode_0440: path_mode_equals(&sudoers_path, MODE_0440.unix),
        sudoers_rule_matches: sudoers_content.contains(&projection.sudoers_rule()),
        sudoers_no_unbounded_helper: !sudoers_content
            .lines()
            .any(|line| line.trim() == unbounded_sudoers_rule_for_projection(projection)),
        sudoers_no_shell: !sudoers_content.contains("/bin/sh") && !sudoers_content.contains("bash"),
        sudoers_no_raw_sudo: !sudoers_content.contains("systemctl")
            && !sudoers_content.contains("apt-get"),
    };

    let mut missing = Vec::new();
    if !checks.helper_exists {
        missing.push("helper_exists".to_string());
    }
    if !checks.helper_mode_0755 {
        missing.push("helper_mode_0755".to_string());
    }
    if !checks.helper_executable {
        missing.push("helper_executable".to_string());
    }
    if !checks.sudoers_exists {
        missing.push("sudoers_exists".to_string());
    }
    if !checks.sudoers_mode_0440 {
        missing.push("sudoers_mode_0440".to_string());
    }
    if !checks.sudoers_rule_matches {
        missing.push("sudoers_rule_matches".to_string());
    }
    if !checks.sudoers_no_unbounded_helper {
        missing.push("sudoers_no_unbounded_helper".to_string());
    }
    if !checks.sudoers_no_shell {
        missing.push("sudoers_no_shell".to_string());
    }
    if !checks.sudoers_no_raw_sudo {
        missing.push("sudoers_no_raw_sudo".to_string());
    }

    HostExecBridgeStatus {
        schema: "cynic.host_exec_bridge.status.v1".to_string(),
        status: if missing.is_empty() {
            "active".to_string()
        } else {
            "missing_activation".to_string()
        },
        helper_path,
        sudoers_target: sudoers_path,
        sudoers_rule: projection.sudoers_rule(),
        helper_sha256,
        sudoers_sha256,
        checks,
        missing,
    }
}

fn file_sha256(path: &str) -> Option<String> {
    fs::read(path)
        .ok()
        .map(|bytes| format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
}

fn unbounded_sudoers_rule_for_projection(projection: &BoundedHelperProjection) -> String {
    format!(
        "{} ALL=(root) NOPASSWD: {}",
        projection.subject, projection.helper_path
    )
}
