//! Lifecycle state persistence (paths + JSON only).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::types::UpdateLifecycleState;

/// Default state path: runtime authority or principal state.
pub fn default_state_path() -> PathBuf {
    if let Ok(p) = std::env::var("CYNIC_LIFECYCLE_STATE") {
        return PathBuf::from(p);
    }
    let authority = PathBuf::from("/var/lib/cynic/lifecycle/update-state.json");
    if Path::new("/var/lib/cynic").is_dir() {
        return authority;
    }
    if let Ok(home) = std::env::var("CYNIC_PRINCIPAL_HOME") {
        return PathBuf::from(home).join(".local/state/cynic/lifecycle/update-state.json");
    }
    if let Some(home) = passwd_home("cynic") {
        return home.join(".local/state/cynic/lifecycle/update-state.json");
    }
    PathBuf::from("var/lib/cynic/lifecycle/update-state.json")
}

fn passwd_home(user: &str) -> Option<PathBuf> {
    let out = Command::new("getent")
        .args(["passwd", user])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&out.stdout);
    let home = line.trim().split(':').nth(5)?;
    if home.is_empty() {
        None
    } else {
        Some(PathBuf::from(home))
    }
}

pub fn load_state(path: &Path) -> UpdateLifecycleState {
    let Ok(raw) = fs::read_to_string(path) else {
        return UpdateLifecycleState::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_state(path: &Path, state: &UpdateLifecycleState) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| format!("write {}: {e}", path.display()))
}
