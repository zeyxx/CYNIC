use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

use cynic_lifecycle::ArgvRunner;
use cynic_protocol::SOURCE_MATERIALIZATION_FORMAT;

use crate::{
    adapters::source_snapshot_store::{digest_name, regular_file_without_symlink, verify_snapshot},
    ports::source_materialization_port::{
        SourceMaterializationPortError, SourceSnapshotAcquisitionPort,
    },
};

const MAX_SNAPSHOT_BYTES: &str = "1073741824";
const MAX_TRANSFER_SECONDS: &str = "300";

#[derive(Debug)]
pub struct HttpsSourceSnapshotAcquisition<'a, R> {
    runner: &'a R,
    snapshot_root: PathBuf,
    base_url: String,
    curl_bin: String,
}

impl<'a, R> HttpsSourceSnapshotAcquisition<'a, R> {
    pub fn new(
        runner: &'a R,
        snapshot_root: impl Into<PathBuf>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            runner,
            snapshot_root: snapshot_root.into(),
            base_url: base_url.into(),
            curl_bin: "curl".to_string(),
        }
    }

    fn source_url(&self, digest_name: &str) -> Result<String, SourceMaterializationPortError> {
        let base = self.base_url.trim_end_matches('/');
        let authority_and_path = base.strip_prefix("https://").ok_or_else(|| {
            error(
                "source_snapshot_transport_insecure",
                "snapshot base URL must use HTTPS",
            )
        })?;
        let authority = authority_and_path.split('/').next().unwrap_or_default();
        if authority.is_empty()
            || authority.contains('@')
            || base.contains('?')
            || base.contains('#')
            || base.bytes().any(|byte| byte.is_ascii_whitespace())
        {
            return Err(error(
                "source_snapshot_base_url_unsafe",
                "snapshot base URL must not contain userinfo, query, fragment, or whitespace",
            ));
        }
        Ok(format!("{base}/{digest_name}.bundle"))
    }

    fn ensure_store(&self) -> Result<(), SourceMaterializationPortError> {
        fs::create_dir_all(&self.snapshot_root)
            .map_err(|failure| error("source_snapshot_store_create_failed", failure.to_string()))?;
        let metadata = fs::symlink_metadata(&self.snapshot_root)
            .map_err(|failure| error("source_snapshot_store_unobservable", failure.to_string()))?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(error(
                "source_snapshot_store_unsafe",
                "snapshot root must be a directory and not a symlink",
            ));
        }
        Ok(())
    }
}

impl<R: ArgvRunner> SourceSnapshotAcquisitionPort for HttpsSourceSnapshotAcquisition<'_, R> {
    fn acquire(
        &self,
        snapshot_digest: &str,
        snapshot_format: &str,
    ) -> Result<(), SourceMaterializationPortError> {
        if snapshot_format != SOURCE_MATERIALIZATION_FORMAT {
            return Err(error(
                "source_snapshot_acquisition_format_unsupported",
                "only git_bundle_v1 is supported",
            ));
        }
        let digest_name = digest_name(snapshot_digest)?;
        let source_url = self.source_url(digest_name)?;
        self.ensure_store()?;

        let final_path = self.snapshot_root.join(digest_name);
        if regular_file_without_symlink(&final_path) {
            return verify_snapshot(&final_path, snapshot_digest);
        }
        if fs::symlink_metadata(&final_path).is_ok() {
            return Err(error(
                "source_snapshot_destination_unsafe",
                "content-addressed destination exists but is not a regular file",
            ));
        }

        let staging_path = self.snapshot_root.join(format!(".{digest_name}.acquiring"));
        let staging = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staging_path)
            .map_err(|failure| {
                error("source_snapshot_staging_create_failed", failure.to_string())
            })?;
        staging.sync_all().map_err(|failure| {
            cleanup_file(
                &staging_path,
                "source_snapshot_staging_sync_failed",
                failure.to_string(),
            )
        })?;
        drop(staging);

        let staging_text = path_text(&staging_path)?;
        let transfer = self.runner.run(&[
            self.curl_bin.as_str(),
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--max-time",
            MAX_TRANSFER_SECONDS,
            "--max-filesize",
            MAX_SNAPSHOT_BYTES,
            "--output",
            staging_text,
            source_url.as_str(),
        ]);
        if !transfer.success() {
            return Err(cleanup_file(
                &staging_path,
                "source_snapshot_transfer_failed",
                transfer.stderr,
            ));
        }
        if !regular_file_without_symlink(&staging_path) {
            return Err(cleanup_file(
                &staging_path,
                "source_snapshot_staging_unsafe",
                "download did not produce a regular file",
            ));
        }
        if let Err(failure) = verify_snapshot(&staging_path, snapshot_digest) {
            return Err(cleanup_file(&staging_path, &failure.code, failure.message));
        }

        fs::hard_link(&staging_path, &final_path).map_err(|failure| {
            cleanup_file(
                &staging_path,
                "source_snapshot_promote_failed",
                failure.to_string(),
            )
        })?;
        fs::remove_file(&staging_path).map_err(|failure| {
            error(
                "source_snapshot_staging_cleanup_failed",
                failure.to_string(),
            )
        })?;
        File::open(&self.snapshot_root)
            .and_then(|directory| directory.sync_all())
            .map_err(|failure| error("source_snapshot_store_sync_failed", failure.to_string()))?;
        verify_snapshot(&final_path, snapshot_digest)
    }
}

fn path_text(path: &Path) -> Result<&str, SourceMaterializationPortError> {
    path.to_str().ok_or_else(|| {
        error(
            "source_snapshot_path_not_utf8",
            "snapshot path is not UTF-8",
        )
    })
}

fn cleanup_file(
    path: &Path,
    code: &str,
    message: impl Into<String>,
) -> SourceMaterializationPortError {
    let suffix = fs::remove_file(path)
        .err()
        .map(|failure| format!("; rollback_failed:{failure}"))
        .unwrap_or_default();
    error(code, format!("{}{suffix}", message.into()))
}

fn error(code: impl Into<String>, message: impl Into<String>) -> SourceMaterializationPortError {
    SourceMaterializationPortError::new(code, message)
}

#[cfg(test)]
mod tests {
    use cynic_lifecycle::MapArgvRunner;
    use sha2::{Digest, Sha256};

    use super::*;

    #[test]
    fn rejects_insecure_transport_before_running_curl() {
        let root = tempfile::tempdir().unwrap();
        let runner = MapArgvRunner::default();
        let adapter =
            HttpsSourceSnapshotAcquisition::new(&runner, root.path(), "http://store.invalid");

        let failure = adapter
            .acquire(
                &format!("sha256:{}", "a".repeat(64)),
                SOURCE_MATERIALIZATION_FORMAT,
            )
            .unwrap_err();

        assert_eq!(failure.code, "source_snapshot_transport_insecure");
    }

    #[test]
    fn rejects_credentials_embedded_in_base_url() {
        let root = tempfile::tempdir().unwrap();
        let runner = MapArgvRunner::default();
        let adapter = HttpsSourceSnapshotAcquisition::new(
            &runner,
            root.path(),
            "https://token@store.invalid/snapshots",
        );

        let failure = adapter
            .acquire(
                &format!("sha256:{}", "a".repeat(64)),
                SOURCE_MATERIALIZATION_FORMAT,
            )
            .unwrap_err();

        assert_eq!(failure.code, "source_snapshot_base_url_unsafe");
    }

    #[test]
    fn accepts_an_existing_blob_only_after_digest_read_back() {
        let root = tempfile::tempdir().unwrap();
        let payload = b"content-addressed-source";
        let digest = format!("sha256:{}", hex::encode(Sha256::digest(payload)));
        let name = digest.strip_prefix("sha256:").unwrap();
        fs::write(root.path().join(name), payload).unwrap();
        let runner = MapArgvRunner::default();
        let adapter =
            HttpsSourceSnapshotAcquisition::new(&runner, root.path(), "https://store.invalid");

        adapter
            .acquire(&digest, SOURCE_MATERIALIZATION_FORMAT)
            .unwrap();
    }
}
