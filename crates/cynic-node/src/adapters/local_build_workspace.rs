use std::{fs, path::Path};

use cynic_process::ArgvRunner;

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
            // H1b: the workspace already exists — try to update it in place
            // (fetch + reset --hard) so unchanged sources keep their old
            // mtimes and cargo fingerprints hit the warm shared target.
            // If the workspace is corrupt (e.g. .git missing), fall back
            // to a clean recreate.
            match self.update_in_place(&layout.workspace_root, source_commit) {
                Ok(()) => false,
                Err(_) => {
                    self.remove_workspace(repository, &layout.workspace_root)?;
                    self.create_worktree(repository, &layout.workspace_root, source_commit)?;
                    true
                }
            }
        } else {
            self.create_worktree(repository, &layout.workspace_root, source_commit)?;
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

impl<R: ArgvRunner> LocalBuildWorkspace<'_, R> {
    /// Create a new git worktree at `workspace` for `source_commit`.
    fn create_worktree(
        &self,
        repository: &str,
        workspace: &std::path::Path,
        source_commit: &str,
    ) -> Result<(), AgentWorkspacePortError> {
        let parent = workspace
            .parent()
            .ok_or_else(|| error("build_workspace_parent_missing"))?;
        fs::create_dir_all(parent)
            .map_err(|_| error("build_workspace_parent_create_failed"))?;
        let metadata = fs::symlink_metadata(parent)
            .map_err(|_| error("build_workspace_parent_unobservable"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(error("build_workspace_parent_rejected"));
        }
        let workspace = path_text(workspace)?;
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
        Ok(())
    }

    /// Update an existing worktree in place: fetch + reset --hard to the
    /// target commit. Returns an error when the workspace is not a valid
    /// git worktree (corrupt), so the caller can fall back to a clean
    /// recreate.
    fn update_in_place(
        &self,
        workspace: &std::path::Path,
        source_commit: &str,
    ) -> Result<(), AgentWorkspacePortError> {
        let workspace = path_text(workspace)?;
        // Verify the workspace is a valid git worktree by reading HEAD.
        let observed = self.runner.run(&[
            self.git_bin,
            "-C",
            workspace,
            "rev-parse",
            "HEAD",
        ]);
        if !observed.success() {
            return Err(error("build_workspace_corrupt"));
        }
        // Fetch the latest refs from origin.
        let fetch = self.runner.run(&[
            self.git_bin,
            "-C",
            workspace,
            "fetch",
            "--quiet",
            "origin",
        ]);
        if !fetch.success() {
            return Err(error("build_workspace_fetch_failed"));
        }
        // Reset the working tree to the target commit.
        let reset = self.runner.run(&[
            self.git_bin,
            "-C",
            workspace,
            "reset",
            "--hard",
            source_commit,
        ]);
        if !reset.success() {
            return Err(error("build_workspace_reset_failed"));
        }
        Ok(())
    }

    /// Remove a corrupt workspace: try `git worktree remove` first, fall
    /// back to a plain recursive removal.
    fn remove_workspace(
        &self,
        repository: &str,
        workspace: &std::path::Path,
    ) -> Result<(), AgentWorkspacePortError> {
        let workspace = path_text(workspace)?;
        let remove = self.runner.run(&[
            self.git_bin,
            "-C",
            repository,
            "worktree",
            "remove",
            "--force",
            workspace,
        ]);
        if !remove.success() {
            fs::remove_dir_all(workspace)
                .map_err(|_| error("build_workspace_remove_failed"))?;
            // Clean up stale worktree registration left by the corrupt tree.
            let _ = self.runner.run(&[
                self.git_bin,
                "-C",
                repository,
                "worktree",
                "prune",
            ]);
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_process::LocalArgvRunner;
    use crate::ports::agent_workspace_port::AgentWorkspaceLayout;

    fn git(args: &[&str], dir: &std::path::Path) -> bool {
        std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    fn init_repo(path: &std::path::Path) {
        std::fs::create_dir_all(path.join(".git")).unwrap();
        git(&["init", "-q"], path);
        git(&["config", "user.email", "t@t"], path);
        git(&["config", "user.name", "t"], path);
        std::fs::write(path.join("f"), b"x").unwrap();
        git(&["add", "f"], path);
        git(&["commit", "-qm", "init"], path);
    }

    fn layout(repository: &std::path::Path, workspace: &std::path::Path) -> AgentWorkspaceLayout {
        AgentWorkspaceLayout {
            repository_root: repository.to_path_buf(),
            workspace_root: workspace.to_path_buf(),
            dedicated: true,
            required_principal: "cynic".to_string(),
            evidence_digest: "sha256:test".to_string(),
            transport_uri: None,
        }
    }

    fn head_sha(repo: &std::path::Path) -> String {
        let out = std::process::Command::new("git")
            .args(["-C", repo.to_str().unwrap(), "rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[test]
    fn second_prepare_same_sha_reuses_checkout() {
        // H1b: second prepare of the same SHA must reuse the existing
        // checkout dir — no re-clone. The .git/worktree metadata must
        // persist and a sentinel file with its old mtime must survive.
        let case = tempfile::tempdir().unwrap();
        let repository = case.path().join("repository");
        let workspace = case.path().join("stable").join("debug");
        init_repo(&repository);
        let commit = head_sha(&repository);
        let runner = LocalArgvRunner;
        let port = LocalBuildWorkspace::new(&runner, "git");
        let layout = layout(&repository, &workspace);
        git(
            &["remote", "add", "origin", repository.to_str().unwrap()],
            &repository,
        );
        // First prepare: creates the worktree.
        let first = port.prepare(&layout, &commit).unwrap();
        assert!(first.created, "first prepare must create the worktree");
        // Add a sentinel file with a known mtime.
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(workspace.join("sentinel.txt"), b"keep-me").unwrap();
        let sentinel = workspace.join("sentinel.txt");
        let mtime_before = std::fs::metadata(&sentinel)
            .unwrap()
            .modified()
            .unwrap();
        // Second prepare: must reuse the existing checkout.
        let second = port.prepare(&layout, &commit).unwrap();
        assert!(
            !second.created,
            "second prepare must NOT re-create the worktree"
        );
        assert!(
            sentinel.exists(),
            "sentinel file must survive second prepare (no wipe)"
        );
        let mtime_after = std::fs::metadata(&sentinel)
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(
            mtime_before, mtime_after,
            "sentinel mtime must be unchanged (no re-materialization)"
        );
        // The .git/worktrees registration must persist.
        assert!(
            repository
                .join(".git")
                .join("worktrees")
                .exists(),
            "worktree registration must persist across prepares"
        );
    }

    #[test]
    fn changed_sha_updates_in_place() {
        // H1b: when the source commit changes, the existing worktree must
        // be updated in place (fetch + reset --hard) — no full re-clone.
        let case = tempfile::tempdir().unwrap();
        let repository = case.path().join("repository");
        let workspace = case.path().join("stable").join("debug");
        init_repo(&repository);
        let runner = LocalArgvRunner;
        let port = LocalBuildWorkspace::new(&runner, "git");
        let layout = layout(&repository, &workspace);
        let first_commit = head_sha(&repository);
        git(
            &["remote", "add", "origin", repository.to_str().unwrap()],
            &repository,
        );
        // First prepare: creates the worktree at the initial commit.
        let first = port.prepare(&layout, &first_commit).unwrap();
        assert!(first.created);
        // Make a new commit in the repository.
        std::fs::write(repository.join("g"), b"y").unwrap();
        git(&["add", "g"], &repository);
        git(&["commit", "-qm", "second"], &repository);
        let new_commit = head_sha(&repository);
        // Second prepare: must update in place.
        let second = port.prepare(&layout, &new_commit).unwrap();
        assert!(
            !second.created,
            "second prepare must NOT re-create the worktree"
        );
        // HEAD must point to the new commit.
        let observed = std::process::Command::new("git")
            .args(["-C", workspace.to_str().unwrap(), "rev-parse", "HEAD"])
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&observed.stdout).trim(),
            new_commit,
            "HEAD must be updated to the new commit"
        );
        // The .git/worktrees registration must persist (no re-clone).
        assert!(
            repository
                .join(".git")
                .join("worktrees")
                .exists(),
            "worktree registration must persist across updates"
        );
    }

    #[test]
    fn corrupt_workspace_falls_back_to_clean_recreate() {
        // H1b: a corrupt workspace (e.g. .git removed) must fall back to
        // a clean recreate — no poison from the previous state.
        let case = tempfile::tempdir().unwrap();
        let repository = case.path().join("repository");
        let workspace = case.path().join("stable").join("debug");
        init_repo(&repository);
        let commit = head_sha(&repository);
        let runner = LocalArgvRunner;
        let port = LocalBuildWorkspace::new(&runner, "git");
        let layout = layout(&repository, &workspace);
        // First prepare: creates the worktree.
        let first = port.prepare(&layout, &commit).unwrap();
        assert!(first.created);
        // Corrupt the workspace by removing .git (a file in a worktree).
        let git_dir = workspace.join(".git");
        if git_dir.is_dir() {
            std::fs::remove_dir_all(&git_dir).unwrap();
        } else if git_dir.exists() {
            std::fs::remove_file(&git_dir).unwrap();
        }
        // Second prepare: must fall back to clean recreate.
        let second = port.prepare(&layout, &commit).unwrap();
        assert!(
            second.created,
            "corrupt workspace must be recreated from scratch"
        );
        // The recreated workspace must be valid.
        let observed = std::process::Command::new("git")
            .args(["-C", workspace.to_str().unwrap(), "rev-parse", "HEAD"])
            .output()
            .unwrap();
        assert!(
            observed.status.success(),
            "recreated workspace must be a valid git worktree"
        );
    }
}
