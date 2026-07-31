use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::ports::source_materialization_port::SourceMaterializationPortError;

pub(crate) fn digest_name(digest: &str) -> Result<&str, SourceMaterializationPortError> {
    digest
        .strip_prefix("sha256:")
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| error("source_snapshot_digest_invalid", "invalid sha256 digest"))
}

pub(crate) fn snapshot_path(
    root: &Path,
    digest: &str,
) -> Result<PathBuf, SourceMaterializationPortError> {
    Ok(root.join(digest_name(digest)?))
}

pub(crate) fn verify_snapshot(
    path: &Path,
    expected: &str,
) -> Result<(), SourceMaterializationPortError> {
    if !regular_file_without_symlink(path) {
        return Err(error(
            "source_snapshot_file_unsafe",
            "snapshot must be a regular file and not a symlink",
        ));
    }
    if sha256_file(path)? == expected {
        Ok(())
    } else {
        Err(error(
            "source_snapshot_acquisition_digest_mismatch",
            "snapshot differs from requested digest",
        ))
    }
}

pub(crate) fn sha256_file(path: &Path) -> Result<String, SourceMaterializationPortError> {
    let mut file = File::open(path)
        .map_err(|failure| error("source_snapshot_read_failed", failure.to_string()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|failure| error("source_snapshot_read_failed", failure.to_string()))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("sha256:{}", hex::encode(hasher.finalize())))
}

pub(crate) fn regular_file_without_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
}

fn error(code: impl Into<String>, message: impl Into<String>) -> SourceMaterializationPortError {
    SourceMaterializationPortError::new(code, message)
}
