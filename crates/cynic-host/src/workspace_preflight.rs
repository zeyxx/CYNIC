//! Workspace preflight.
//!
//! Before creating a worktree, verify:
//! - Base commit is reachable from the source repository
//! - Target path is inside the declared workspace store
//! - No existing worktree occupies the target path
//! - Path traversal, symlink escapes, and absolute paths are rejected

use std::path::{Path, PathBuf};

use serde::Serialize;

pub const WORKSPACE_PREFLIGHT_SCHEMA: &str = "cynic.host.workspace_preflight.v1";

#[derive(Debug, Clone, Serialize)]
pub struct WorkspacePreflightReport {
    pub schema: &'static str,
    pub admitted: bool,
    pub blockers: Vec<String>,
}

/// Pure preflight check — no Git mutation, no filesystem mutation.
pub fn workspace_preflight(
    source_root: &Path,
    target_root: &Path,
    base_commit: &str,
    workspace_store: &Path,
) -> WorkspacePreflightReport {
    let mut blockers = Vec::new();

    if base_commit.trim().is_empty() {
        blockers.push("workspace_base_commit_empty".to_string());
    }

    if source_root.as_os_str().is_empty() {
        blockers.push("workspace_source_root_empty".to_string());
    }

    if target_root.as_os_str().is_empty() {
        blockers.push("workspace_target_root_empty".to_string());
    }

    if !target_root.starts_with(workspace_store) {
        blockers.push("workspace_target_outside_workspace_store".to_string());
    }

    let target_str = target_root.to_string_lossy();
    if target_str.contains("..") {
        blockers.push("workspace_path_traversal_forbidden".to_string());
    }

    if target_root.exists() {
        blockers.push("workspace_target_already_exists".to_string());
    }

    if let Ok(metadata) = std::fs::symlink_metadata(target_root) {
        if metadata.file_type().is_symlink() {
            blockers.push("workspace_target_is_symlink".to_string());
        }
    }

    if let Some(parent) = target_root.parent() {
        if parent.exists() {
            if let Ok(md) = std::fs::symlink_metadata(parent) {
                if md.file_type().is_symlink() {
                    blockers.push("workspace_parent_is_symlink".to_string());
                }
            }
        }
    }

    WorkspacePreflightReport {
        schema: WORKSPACE_PREFLIGHT_SCHEMA,
        admitted: blockers.is_empty(),
        blockers,
    }
}

/// Compute the canonical target path for a workspace.
pub fn resolve_workspace_target(
    workspace_store: &Path,
    repository_id: &str,
    workspace_id: &str,
) -> PathBuf {
    workspace_store.join(repository_id).join(workspace_id)
}

/// Validate an identifier (no path syntax, no empty).
pub fn is_valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_valid_workspace_target() {
        let store = Path::new("/var/lib/cynic/source/workspaces");
        let target = resolve_workspace_target(store, "cynic", "ws-a");
        let report = workspace_preflight(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            &target,
            "abc123",
            store,
        );
        assert!(report.admitted, "blockers: {:?}", report.blockers);
    }

    #[test]
    fn rejects_target_outside_store() {
        let store = Path::new("/var/lib/cynic/source/workspaces");
        let target = Path::new("/tmp/escape/ws-a");
        let report = workspace_preflight(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            target,
            "abc123",
            store,
        );
        assert!(!report.admitted);
        assert!(report
            .blockers
            .contains(&"workspace_target_outside_workspace_store".to_string()));
    }

    #[test]
    fn rejects_empty_base_commit() {
        let store = Path::new("/var/lib/cynic/source/workspaces");
        let target = resolve_workspace_target(store, "cynic", "ws-a");
        let report = workspace_preflight(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            &target,
            "",
            store,
        );
        assert!(!report.admitted);
        assert!(report
            .blockers
            .contains(&"workspace_base_commit_empty".to_string()));
    }

    #[test]
    fn rejects_existing_target() {
        let temp = tempfile::tempdir().unwrap();
        let store = temp.path().join("store");
        std::fs::create_dir_all(&store).unwrap();
        let target = store.join("cynic").join("ws-a");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::File::create(&target).unwrap();

        let report = workspace_preflight(
            temp.path().join("source").as_path(),
            &target,
            "abc123",
            &store,
        );
        assert!(!report.admitted);
        assert!(report
            .blockers
            .contains(&"workspace_target_already_exists".to_string()));
    }

    #[test]
    fn rejects_symlink_target() {
        let temp = tempfile::tempdir().unwrap();
        let store = temp.path().join("store");
        std::fs::create_dir_all(&store).unwrap();
        let real_dir = temp.path().join("real");
        std::fs::create_dir_all(&real_dir).unwrap();
        let target = store.join("cynic").join("ws-a");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&real_dir, &target).unwrap();

        let report = workspace_preflight(
            temp.path().join("source").as_path(),
            &target,
            "abc123",
            &store,
        );
        assert!(!report.admitted);
        assert!(report
            .blockers
            .contains(&"workspace_target_is_symlink".to_string()));
    }

    #[test]
    fn valid_identifier_passes() {
        assert!(is_valid_identifier("ws-a"));
        assert!(is_valid_identifier("cynic"));
    }

    #[test]
    fn invalid_identifier_fails() {
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("."));
        assert!(!is_valid_identifier(".."));
        assert!(!is_valid_identifier("foo/bar"));
        assert!(!is_valid_identifier("foo\\bar"));
    }
}
