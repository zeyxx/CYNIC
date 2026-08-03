use std::{fs, path::Path};

use cynic_host::update_lifecycle::ArgvRunner;

use crate::ports::{
    agent_workspace_port::{AgentWorkspaceLayout, AgentWorkspacePortError},
    build_scheduler_port::{BuildWorkspacePort, BuildWorkspaceReadBack},
};

#[derive(Debug)]
pub struct LocalBuildWorkspace<'a, R> {
    runner: &'a R,
    git_bin: &'a str,
}

impl<'a, R> LocalBuildWorkspace<'a, R> {
    pub fn new(runner: &'a R, git_bin: &'a str) -> Self {
        Self { runner, git_bin }
    }
}

impl<R: ArgvRunner> BuildWorkspacePort for LocalBuildWorkspace<'_, R> {
    fn prepare(
        &self,
        layout: &AgentWorkspaceLayout,
        source_commit: &str,
    ) -> Result<BuildWorkspaceReadBack, AgentWorkspacePortError> {
        if !layout.dedicated {
            return Err(error("build_workspace_shared_layout"));
        }
        let repository = canonical_directory(&layout.repository_root, "build_repository")?;
        let repository = path_text(&repository)?;
        let created = if layout.workspace_root.exists() {
            false
        } else {
            let parent = layout
                .workspace_root
                .parent()
                .ok_or_else(|| error("build_workspace_parent_missing"))?;
            fs::create_dir_all(parent)
                .map_err(|_| error("build_workspace_parent_create_failed"))?;
            let metadata = fs::symlink_metadata(parent)
                .map_err(|_| error("build_workspace_parent_unobservable"))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(error("build_workspace_parent_rejected"));
            }
            let workspace = path_text(&layout.workspace_root)?;
            let create = self.runner.run(&[
                self.git_bin,
                "-C",
                repository,
                "worktree",
                "add",
                "--detach",
                workspace,
                source_commit,
            ]);
            if !create.success() {
                return Err(AgentWorkspacePortError::new(
                    "build_workspace_create_failed",
                    create
                        .spawn_error
                        .as_deref()
                        .unwrap_or(create.stderr.trim()),
                ));
            }
            true
        };
        let workspace = canonical_directory(&layout.workspace_root, "build_workspace")?;
        let observed = self.runner.run(&[
            self.git_bin,
            "-C",
            path_text(&workspace)?,
            "rev-parse",
            "HEAD",
        ]);
        if !observed.success() || observed.stdout.trim() != source_commit {
            return Err(error("build_workspace_source_read_back_mismatch"));
        }
        Ok(BuildWorkspaceReadBack {
            workspace_root: workspace,
            source_commit: source_commit.to_string(),
            created,
        })
    }
}

fn canonical_directory(
    path: &Path,
    code: &str,
) -> Result<std::path::PathBuf, AgentWorkspacePortError> {
    let original = fs::symlink_metadata(path).map_err(|_| error(&format!("{code}_unavailable")))?;
    if original.file_type().is_symlink() || !original.is_dir() {
        return Err(error(&format!("{code}_rejected")));
    }
    let canonical = fs::canonicalize(path).map_err(|_| error(&format!("{code}_unavailable")))?;
    let metadata =
        fs::symlink_metadata(&canonical).map_err(|_| error(&format!("{code}_unobservable")))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(error(&format!("{code}_rejected")));
    }
    Ok(canonical)
}

fn path_text(path: &Path) -> Result<&str, AgentWorkspacePortError> {
    path.to_str()
        .ok_or_else(|| error("build_workspace_path_not_utf8"))
}

fn error(code: &str) -> AgentWorkspacePortError {
    AgentWorkspacePortError::new(code, code)
}
