use std::{
    fs,
    path::{Path, PathBuf},
};

use cynic_host::update_lifecycle::ArgvRunner;
use cynic_protocol::{
    SOURCE_REPOSITORY_BOOTSTRAP_OBSERVATION_SCHEMA, SourceRepositoryBootstrapObservation,
    SourceRepositoryBootstrapRequest,
};

use crate::ports::{
    agent_workspace_port::AgentWorkspaceLayout,
    source_repository_bootstrap_port::{
        SourceRepositoryBootstrapPort, SourceRepositoryBootstrapPortError,
    },
};

#[derive(Debug)]
pub struct LocalSourceRepositoryBootstrap<'a, R> {
    runner: &'a R,
    state_root: PathBuf,
    git_bin: &'a str,
    id_bin: &'a str,
}

impl<'a, R> LocalSourceRepositoryBootstrap<'a, R> {
    pub fn new(
        runner: &'a R,
        state_root: impl Into<PathBuf>,
        git_bin: &'a str,
        id_bin: &'a str,
    ) -> Self {
        Self {
            runner,
            state_root: state_root.into(),
            git_bin,
            id_bin,
        }
    }
}

impl<R: ArgvRunner> SourceRepositoryBootstrapPort for LocalSourceRepositoryBootstrap<'_, R> {
    fn observe(
        &self,
        layout: &AgentWorkspaceLayout,
        request: &SourceRepositoryBootstrapRequest,
    ) -> Result<SourceRepositoryBootstrapObservation, SourceRepositoryBootstrapPortError> {
        let required_uid = observed_uid(self.runner, self.id_bin, &layout.required_principal)?;
        let current_uid = observed_current_uid(self.runner, self.id_bin)?;
        let state_metadata = fs::symlink_metadata(&self.state_root).ok();
        let state_root_exists = state_metadata.is_some();
        let state_root_is_directory = state_metadata
            .as_ref()
            .is_some_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink());
        let state_root_owned_by_principal = state_metadata
            .as_ref()
            .and_then(metadata_uid)
            .is_some_and(|uid| uid == required_uid);

        let repository_metadata = fs::symlink_metadata(&layout.repository_root).ok();
        let repository_exists = repository_metadata.is_some();
        let repository_path = path_text(&layout.repository_root, "repository_root")?.to_string();
        let mut unknowns = Vec::new();
        let mut repository_is_git = false;
        let mut observed_revision = None;
        let mut observed_source = None;
        if let Some(metadata) = repository_metadata {
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                let git = self.runner.run(&[
                    self.git_bin,
                    "-C",
                    &repository_path,
                    "rev-parse",
                    "--is-inside-work-tree",
                ]);
                repository_is_git = git.success() && git.stdout.trim() == "true";
                if repository_is_git {
                    let revision = self.runner.run(&[
                        self.git_bin,
                        "-C",
                        &repository_path,
                        "rev-parse",
                        "HEAD",
                    ]);
                    if revision.success() {
                        observed_revision = Some(revision.stdout.trim().to_string());
                    } else {
                        unknowns.push("source_bootstrap_revision_unobservable".to_string());
                    }
                    let source = self.runner.run(&[
                        self.git_bin,
                        "-C",
                        &repository_path,
                        "remote",
                        "get-url",
                        "origin",
                    ]);
                    if source.success() {
                        observed_source = Some(source.stdout.trim().to_string());
                    } else {
                        unknowns.push("source_bootstrap_source_unobservable".to_string());
                    }
                }
            }
        }

        Ok(SourceRepositoryBootstrapObservation {
            schema: SOURCE_REPOSITORY_BOOTSTRAP_OBSERVATION_SCHEMA.to_string(),
            node_id: request.node_id.clone(),
            repository_id: request.repository_id.clone(),
            repository_path,
            required_principal: layout.required_principal.clone(),
            state_root_exists,
            state_root_is_directory,
            state_root_owned_by_principal,
            running_as_required_principal: current_uid == required_uid,
            repository_exists,
            repository_is_git,
            observed_revision,
            observed_source,
            unknowns,
        })
    }

    fn apply(
        &self,
        layout: &AgentWorkspaceLayout,
        request: &SourceRepositoryBootstrapRequest,
    ) -> Result<(), SourceRepositoryBootstrapPortError> {
        if layout.repository_root.exists() {
            return sync_existing_repository(self.runner, self.git_bin, layout, request);
        }
        let parent = layout.repository_root.parent().ok_or_else(|| {
            error(
                "source_bootstrap_repository_parent_missing",
                "repository root has no parent",
            )
        })?;
        fs::create_dir_all(parent).map_err(|failure| {
            error(
                "source_bootstrap_repository_parent_failed",
                format!("cannot create repository parent: {failure}"),
            )
        })?;
        let parent_metadata = fs::symlink_metadata(parent).map_err(|failure| {
            error(
                "source_bootstrap_repository_parent_unobservable",
                format!("cannot observe repository parent: {failure}"),
            )
        })?;
        if !parent_metadata.is_dir() || parent_metadata.file_type().is_symlink() {
            return Err(error(
                "source_bootstrap_repository_parent_rejected",
                "repository parent is a symlink or not a directory",
            ));
        }

        let staging = staging_path(&layout.repository_root, &request.repository_id)?;
        if staging.exists() {
            return Err(error(
                "source_bootstrap_staging_exists",
                format!("staging path already exists: {}", staging.display()),
            ));
        }
        let staging_text = path_text(&staging, "staging_path")?;
        let clone = self.runner.run(&[
            self.git_bin,
            "clone",
            "--no-checkout",
            "--origin",
            "origin",
            &request.source,
            staging_text,
        ]);
        if !clone.success() {
            return Err(rollback_error(
                &staging,
                "source_bootstrap_clone_failed",
                clone.spawn_error.as_deref().unwrap_or(clone.stderr.trim()),
            ));
        }
        let checkout = self.runner.run(&[
            self.git_bin,
            "-C",
            staging_text,
            "checkout",
            "--detach",
            &request.revision,
        ]);
        if !checkout.success() {
            return Err(rollback_error(
                &staging,
                "source_bootstrap_checkout_failed",
                checkout
                    .spawn_error
                    .as_deref()
                    .unwrap_or(checkout.stderr.trim()),
            ));
        }
        let read_back = self
            .runner
            .run(&[self.git_bin, "-C", staging_text, "rev-parse", "HEAD"]);
        if !read_back.success() || read_back.stdout.trim() != request.revision {
            return Err(rollback_error(
                &staging,
                "source_bootstrap_revision_read_back_failed",
                "staging checkout did not read back the exact requested revision",
            ));
        }
        fs::rename(&staging, &layout.repository_root).map_err(|failure| {
            rollback_error(
                &staging,
                "source_bootstrap_promote_failed",
                &format!("cannot atomically promote staging repository: {failure}"),
            )
        })
    }
}

fn sync_existing_repository<R: ArgvRunner>(
    runner: &R,
    git_bin: &str,
    layout: &AgentWorkspaceLayout,
    request: &SourceRepositoryBootstrapRequest,
) -> Result<(), SourceRepositoryBootstrapPortError> {
    let metadata = fs::symlink_metadata(&layout.repository_root).map_err(|failure| {
        error(
            "source_sync_repository_unobservable",
            format!("cannot observe managed repository: {failure}"),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(error(
            "source_sync_repository_rejected",
            "managed repository is a symlink or not a directory",
        ));
    }
    let repository = path_text(&layout.repository_root, "repository_root")?;
    let source = runner.run(&[git_bin, "-C", repository, "remote", "get-url", "origin"]);
    if !source.success() || source.stdout.trim() != request.source {
        return Err(error(
            "source_sync_origin_mismatch",
            "managed repository origin does not match desired source authority",
        ));
    }
    let before = runner.run(&[git_bin, "-C", repository, "rev-parse", "HEAD"]);
    let before_revision = before.success().then(|| before.stdout.trim().to_string());
    let fetch = runner.run(&[
        git_bin,
        "-C",
        repository,
        "fetch",
        "--prune",
        "--no-tags",
        "origin",
    ]);
    if !fetch.success() {
        return Err(error(
            "source_sync_fetch_failed",
            fetch.spawn_error.as_deref().unwrap_or(fetch.stderr.trim()),
        ));
    }
    let checkout = runner.run(&[
        git_bin,
        "-C",
        repository,
        "checkout",
        "--detach",
        &request.revision,
    ]);
    if !checkout.success() {
        return Err(error(
            "source_sync_checkout_failed",
            checkout
                .spawn_error
                .as_deref()
                .unwrap_or(checkout.stderr.trim()),
        ));
    }
    let read_back = runner.run(&[git_bin, "-C", repository, "rev-parse", "HEAD"]);
    if read_back.success() && read_back.stdout.trim() == request.revision {
        return Ok(());
    }
    if let Some(before_revision) = before_revision {
        let _ = runner.run(&[
            git_bin,
            "-C",
            repository,
            "checkout",
            "--detach",
            &before_revision,
        ]);
    }
    Err(error(
        "source_sync_revision_read_back_failed",
        "managed repository did not read back the exact requested revision",
    ))
}

fn observed_uid<R: ArgvRunner>(
    runner: &R,
    id_bin: &str,
    principal: &str,
) -> Result<u32, SourceRepositoryBootstrapPortError> {
    let output = runner.run(&[id_bin, "-u", principal]);
    parse_uid(output.success(), output.stdout.trim(), "required principal")
}

fn observed_current_uid<R: ArgvRunner>(
    runner: &R,
    id_bin: &str,
) -> Result<u32, SourceRepositoryBootstrapPortError> {
    let output = runner.run(&[id_bin, "-u"]);
    parse_uid(output.success(), output.stdout.trim(), "current principal")
}

fn parse_uid(
    success: bool,
    value: &str,
    subject: &str,
) -> Result<u32, SourceRepositoryBootstrapPortError> {
    if !success {
        return Err(error(
            "source_bootstrap_principal_unobservable",
            format!("cannot observe {subject} UID"),
        ));
    }
    value.parse().map_err(|failure| {
        error(
            "source_bootstrap_principal_uid_invalid",
            format!("{subject} UID is invalid: {failure}"),
        )
    })
}

#[cfg(unix)]
fn metadata_uid(metadata: &fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    Some(metadata.uid())
}

#[cfg(not(unix))]
fn metadata_uid(_metadata: &fs::Metadata) -> Option<u32> {
    None
}

fn staging_path(
    repository_root: &Path,
    repository_id: &str,
) -> Result<PathBuf, SourceRepositoryBootstrapPortError> {
    let parent = repository_root.parent().ok_or_else(|| {
        error(
            "source_bootstrap_repository_parent_missing",
            "repository root has no parent",
        )
    })?;
    Ok(parent.join(format!(".{repository_id}.source-bootstrap")))
}

fn rollback_error(staging: &Path, code: &str, message: &str) -> SourceRepositoryBootstrapPortError {
    let rollback = if staging.exists() {
        fs::remove_dir_all(staging)
            .err()
            .map(|failure| format!("; rollback failed: {failure}"))
            .unwrap_or_default()
    } else {
        String::new()
    };
    error(code, format!("{message}{rollback}"))
}

fn path_text<'a>(
    path: &'a Path,
    field: &str,
) -> Result<&'a str, SourceRepositoryBootstrapPortError> {
    path.to_str().ok_or_else(|| {
        error(
            "source_bootstrap_path_not_utf8",
            format!("{field} cannot be represented as UTF-8 argv"),
        )
    })
}

fn error(
    code: impl Into<String>,
    message: impl Into<String>,
) -> SourceRepositoryBootstrapPortError {
    SourceRepositoryBootstrapPortError::new(code, message)
}
