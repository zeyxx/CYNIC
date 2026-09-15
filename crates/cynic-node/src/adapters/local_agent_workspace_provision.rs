use std::{fs, path::Path};

use cynic_host::update_lifecycle::ArgvRunner;
use cynic_workload::AgentWorkspaceRequest;

use crate::ports::agent_workspace_port::{
    AgentWorkspaceLayout, AgentWorkspacePortError, AgentWorkspaceProvisionPort,
};

#[derive(Debug)]
pub struct LocalAgentWorkspaceProvisioner<'a, R> {
    runner: &'a R,
    git_bin: &'a str,
}

impl<'a, R> LocalAgentWorkspaceProvisioner<'a, R> {
    pub fn new(runner: &'a R, git_bin: &'a str) -> Self {
        Self { runner, git_bin }
    }
}

impl<R: ArgvRunner> AgentWorkspaceProvisionPort for LocalAgentWorkspaceProvisioner<'_, R> {
    fn ensure(
        &self,
        layout: &AgentWorkspaceLayout,
        request: &AgentWorkspaceRequest,
    ) -> Result<bool, AgentWorkspacePortError> {
        if !layout.dedicated {
            return Err(error(
                "workspace_provision_shared_layout",
                "agent workspaces require a dedicated worktree layout",
            ));
        }
        if layout.workspace_root.exists() {
            return Ok(false);
        }
        let repository = canonical_directory(&layout.repository_root, "repository_root")?;
        let repository = path_text(&repository, "repository_root")?;
        let workspace_parent = layout.workspace_root.parent().ok_or_else(|| {
            error(
                "workspace_provision_parent_missing",
                "workspace root has no parent directory",
            )
        })?;
        fs::create_dir_all(workspace_parent).map_err(|failure| {
            error(
                "workspace_provision_parent_failed",
                format!("cannot create workspace parent: {failure}"),
            )
        })?;
        let parent_metadata = fs::symlink_metadata(workspace_parent).map_err(|failure| {
            error(
                "workspace_provision_parent_unobservable",
                format!("cannot observe workspace parent: {failure}"),
            )
        })?;
        if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
            return Err(error(
                "workspace_provision_parent_rejected",
                "workspace parent is a symlink or not a directory",
            ));
        }

        let base_spec = format!("{}^{{commit}}", request.base_commit);
        let verify = self.runner.run(&[
            self.git_bin,
            "-C",
            repository,
            "rev-parse",
            "--verify",
            &base_spec,
        ]);
        if !verify.success() || verify.stdout.trim() != request.base_commit {
            return Err(error(
                "workspace_base_commit_unavailable",
                "managed repository cannot read back the exact requested base commit",
            ));
        }

        let workspace = path_text(&layout.workspace_root, "workspace_root")?;
        let create = self.runner.run(&[
            self.git_bin,
            "-C",
            repository,
            "worktree",
            "add",
            "--detach",
            workspace,
            &request.base_commit,
        ]);
        if !create.success() {
            return Err(error(
                "workspace_provision_git_failed",
                format!(
                    "git worktree creation failed: {}",
                    create
                        .spawn_error
                        .as_deref()
                        .unwrap_or(create.stderr.trim())
                ),
            ));
        }
        Ok(true)
    }
}

fn canonical_directory<'a>(
    path: &'a Path,
    field: &str,
) -> Result<std::path::PathBuf, AgentWorkspacePortError> {
    let canonical = fs::canonicalize(path).map_err(|failure| {
        error(
            "workspace_provision_repository_unavailable",
            format!("{field} cannot be canonicalized: {failure}"),
        )
    })?;
    if !canonical.is_dir() {
        return Err(error(
            "workspace_provision_repository_rejected",
            format!("{field} is not a directory"),
        ));
    }
    Ok(canonical)
}

fn path_text<'a>(path: &'a Path, field: &str) -> Result<&'a str, AgentWorkspacePortError> {
    path.to_str().ok_or_else(|| {
        error(
            "workspace_provision_path_not_utf8",
            format!("{field} cannot be represented as UTF-8 argv"),
        )
    })
}

fn error(code: impl Into<String>, message: impl Into<String>) -> AgentWorkspacePortError {
    AgentWorkspacePortError::new(code, message)
}
