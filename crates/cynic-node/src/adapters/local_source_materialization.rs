use std::{
    fs::{self, File},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    adapters::{
        source_materialization_marker::{
            MARKER, MaterializationMarker, persist_marker, read_marker,
        },
        source_snapshot_store::{
            regular_file_without_symlink, sha256_file, snapshot_path, verify_snapshot,
        },
    },
    ports::source_materialization_port::{
        SourceMaterializationPort, SourceMaterializationPortError,
    },
};
use cynic_lifecycle::ArgvRunner;
use cynic_protocol::{
    SOURCE_MATERIALIZATION_FORMAT, SOURCE_MATERIALIZATION_OBSERVATION_SCHEMA,
    SecurityEvidenceState, SourceMaterializationObservation, SourceMaterializationRequest,
};

#[derive(Debug)]
pub struct LocalSourceMaterialization<'a, R> {
    runner: &'a R,
    state_root: PathBuf,
    snapshot_root: PathBuf,
    expected_node_id: String,
    declared_role: String,
    git_bin: String,
    id_bin: String,
    hostname_bin: String,
}

impl<'a, R> LocalSourceMaterialization<'a, R> {
    pub fn new(
        runner: &'a R,
        state_root: impl Into<PathBuf>,
        snapshot_root: impl Into<PathBuf>,
        expected_node_id: impl Into<String>,
        declared_role: impl Into<String>,
    ) -> Self {
        Self {
            runner,
            state_root: state_root.into(),
            snapshot_root: snapshot_root.into(),
            expected_node_id: expected_node_id.into(),
            declared_role: declared_role.into(),
            git_bin: "git".to_string(),
            id_bin: "id".to_string(),
            hostname_bin: "hostname".to_string(),
        }
    }

    fn snapshot_path(&self, digest: &str) -> Result<PathBuf, SourceMaterializationPortError> {
        snapshot_path(&self.snapshot_root, digest)
    }

    fn target_path(&self, relative: &str) -> Result<PathBuf, SourceMaterializationPortError> {
        let path = Path::new(relative);
        if path.is_absolute()
            || path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(error(
                "source_materialization_target_path_unsafe",
                "target must contain only normal relative path components",
            ));
        }
        Ok(self.state_root.join(path))
    }
}

impl<R: ArgvRunner> SourceMaterializationPort for LocalSourceMaterialization<'_, R> {
    fn observe(
        &self,
        request: &SourceMaterializationRequest,
    ) -> Result<SourceMaterializationObservation, SourceMaterializationPortError> {
        let mut unknowns = Vec::new();
        let hostname = self.runner.run(&[self.hostname_bin.as_str()]);
        let node_identity = if hostname.success()
            && hostname.stdout.trim() == self.expected_node_id
            && request.target_node_id == self.expected_node_id
        {
            SecurityEvidenceState::Proven
        } else {
            unknowns.push("source_materialization_hostname_unproved".to_string());
            SecurityEvidenceState::Unknown
        };
        let principal = self.runner.run(&[self.id_bin.as_str(), "-un"]);
        let running_as_required_principal =
            if principal.success() && principal.stdout.trim() == request.required_principal {
                SecurityEvidenceState::Proven
            } else {
                SecurityEvidenceState::Absent
            };

        let snapshot_path = self.snapshot_path(&request.snapshot_digest)?;
        let snapshot_present = regular_file_without_symlink(&snapshot_path);
        let snapshot_digest_read_back = if snapshot_present {
            match sha256_file(&snapshot_path) {
                Ok(value) => Some(value),
                Err(failure) => {
                    unknowns.push(failure.code);
                    None
                }
            }
        } else {
            None
        };
        let snapshot_format_read_back =
            snapshot_present.then(|| SOURCE_MATERIALIZATION_FORMAT.to_string());

        let target = self.target_path(&request.target_relative_path)?;
        let target_metadata = fs::symlink_metadata(&target).ok();
        let target_exists = target_metadata.is_some();
        let target_is_isolated = if target_metadata
            .as_ref()
            .is_some_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
        {
            SecurityEvidenceState::Proven
        } else if target_exists {
            SecurityEvidenceState::Absent
        } else {
            SecurityEvidenceState::Proven
        };
        let marker = read_marker(&target.join(MARKER)).ok();
        let observed_snapshot_digest = marker.as_ref().map(|marker| marker.snapshot_digest.clone());
        let target_text = path_text(&target, "target")?;
        let observed_commit = if target_exists {
            let output = self.runner.run(&[
                self.git_bin.as_str(),
                "-C",
                target_text,
                "rev-parse",
                "HEAD",
            ]);
            if output.success() {
                Some(output.stdout.trim().to_string())
            } else {
                unknowns.push("source_materialization_commit_unobservable".to_string());
                None
            }
        } else {
            None
        };
        let observed_tree = if target_exists {
            let output = self.runner.run(&[
                self.git_bin.as_str(),
                "-C",
                target_text,
                "rev-parse",
                "HEAD^{tree}",
            ]);
            if output.success() {
                Some(output.stdout.trim().to_string())
            } else {
                unknowns.push("source_materialization_tree_unobservable".to_string());
                None
            }
        } else {
            None
        };

        Ok(SourceMaterializationObservation {
            schema: SOURCE_MATERIALIZATION_OBSERVATION_SCHEMA.to_string(),
            observed_at_unix_ms: now_unix_ms(),
            target_node_id: request.target_node_id.clone(),
            target_role: Some(self.declared_role.clone()),
            node_identity,
            running_as_required_principal,
            snapshot_present,
            snapshot_digest_read_back,
            snapshot_format_read_back,
            target_exists,
            target_is_isolated,
            observed_commit,
            observed_tree,
            observed_snapshot_digest,
            evidence_refs: vec![
                format!("node:{}", self.expected_node_id),
                format!("snapshot:{}", request.snapshot_digest),
            ],
            unknowns,
        })
    }

    fn materialize(
        &self,
        request: &SourceMaterializationRequest,
    ) -> Result<(), SourceMaterializationPortError> {
        if request.release_id.is_empty()
            || request.release_id.len() > 128
            || !request
                .release_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(error(
                "source_materialization_release_id_unsafe",
                "release id is not a safe staging identifier",
            ));
        }
        if request.snapshot_format != SOURCE_MATERIALIZATION_FORMAT {
            return Err(error(
                "source_materialization_format_unsupported",
                "only git_bundle_v1 is supported",
            ));
        }
        let snapshot = self.snapshot_path(&request.snapshot_digest)?;
        if verify_snapshot(&snapshot, &request.snapshot_digest).is_err() {
            return Err(error(
                "source_materialization_snapshot_unproved",
                "snapshot is missing, unsafe, or differs from its digest",
            ));
        }
        let target = self.target_path(&request.target_relative_path)?;
        if target.exists() {
            return Err(error(
                "source_materialization_target_exists",
                "refusing to replace an existing materialization",
            ));
        }
        let parent = target.parent().ok_or_else(|| {
            error(
                "source_materialization_parent_missing",
                "target has no parent",
            )
        })?;
        fs::create_dir_all(parent).map_err(|failure| {
            error(
                "source_materialization_parent_create_failed",
                failure.to_string(),
            )
        })?;
        let parent_metadata = fs::symlink_metadata(parent).map_err(|failure| {
            error(
                "source_materialization_parent_unobservable",
                failure.to_string(),
            )
        })?;
        if !parent_metadata.is_dir() || parent_metadata.file_type().is_symlink() {
            return Err(error(
                "source_materialization_parent_unsafe",
                "target parent is not an isolated directory",
            ));
        }
        let staging = parent.join(format!(".{}.materializing", request.release_id));
        if staging.exists() {
            return Err(error(
                "source_materialization_staging_exists",
                "staging path already exists",
            ));
        }
        let snapshot_text = path_text(&snapshot, "snapshot")?;
        let staging_text = path_text(&staging, "staging")?;
        let clone = self.runner.run(&[
            self.git_bin.as_str(),
            "clone",
            "--no-checkout",
            snapshot_text,
            staging_text,
        ]);
        if !clone.success() {
            return Err(rollback(
                &staging,
                "source_materialization_clone_failed",
                &clone.stderr,
            ));
        }
        let checkout = self.runner.run(&[
            self.git_bin.as_str(),
            "-C",
            staging_text,
            "checkout",
            "--detach",
            request.source_commit.as_str(),
        ]);
        if !checkout.success() {
            return Err(rollback(
                &staging,
                "source_materialization_checkout_failed",
                &checkout.stderr,
            ));
        }
        let read_back = self.runner.run(&[
            self.git_bin.as_str(),
            "-C",
            staging_text,
            "rev-parse",
            "HEAD",
        ]);
        if !read_back.success() || read_back.stdout.trim() != request.source_commit {
            return Err(rollback(
                &staging,
                "source_materialization_commit_read_back_failed",
                "checkout HEAD differs from requested commit",
            ));
        }
        persist_marker(
            &staging.join(MARKER),
            &MaterializationMarker {
                schema: "cynic.source.materialization_marker.v1".to_string(),
                release_id: request.release_id.clone(),
                source_commit: request.source_commit.clone(),
                snapshot_digest: request.snapshot_digest.clone(),
                snapshot_format: request.snapshot_format.clone(),
            },
        )?;
        fs::rename(&staging, &target).map_err(|failure| {
            rollback(
                &staging,
                "source_materialization_promote_failed",
                &failure.to_string(),
            )
        })?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|failure| {
                error(
                    "source_materialization_parent_sync_failed",
                    failure.to_string(),
                )
            })
    }
}

fn rollback(path: &Path, code: &str, message: &str) -> SourceMaterializationPortError {
    let suffix = fs::remove_dir_all(path)
        .err()
        .map(|failure| format!("; rollback_failed:{failure}"))
        .unwrap_or_default();
    error(code, format!("{message}{suffix}"))
}

fn path_text<'a>(path: &'a Path, field: &str) -> Result<&'a str, SourceMaterializationPortError> {
    path.to_str().ok_or_else(|| {
        error(
            "source_materialization_path_not_utf8",
            format!("{field} is not UTF-8"),
        )
    })
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

fn error(code: impl Into<String>, message: impl Into<String>) -> SourceMaterializationPortError {
    SourceMaterializationPortError::new(code, message)
}

#[cfg(test)]
mod tests {
    use cynic_lifecycle::MapArgvRunner;
    use cynic_protocol::{SOURCE_MATERIALIZATION_FORMAT, SOURCE_MATERIALIZATION_REQUEST_SCHEMA};

    use super::*;

    #[test]
    fn rejects_release_id_that_can_escape_staging_name() {
        let root = tempfile::tempdir().unwrap();
        let runner = MapArgvRunner::default();
        let adapter = LocalSourceMaterialization::new(
            &runner,
            root.path(),
            root.path().join("blobs"),
            "cynic-builder",
            "build_runner",
        );
        let request = SourceMaterializationRequest {
            schema: SOURCE_MATERIALIZATION_REQUEST_SCHEMA.to_string(),
            release_id: "../escape".to_string(),
            repository_id: "cynic".to_string(),
            source_commit: "a".repeat(40),
            source_tree: None,
            snapshot_digest: format!("sha256:{}", "b".repeat(64)),
            snapshot_format: SOURCE_MATERIALIZATION_FORMAT.to_string(),
            target_node_id: "cynic-builder".to_string(),
            target_relative_path: "source/releases/release-1".to_string(),
            required_principal: "cynic".to_string(),
            retention_until_unix_ms: u64::MAX,
        };

        let failure = adapter.materialize(&request).unwrap_err();

        assert_eq!(failure.code, "source_materialization_release_id_unsafe");
    }
}
