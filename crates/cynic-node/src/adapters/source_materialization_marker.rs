use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::ports::source_materialization_port::SourceMaterializationPortError;

pub(crate) const MARKER: &str = ".cynic-source-materialization.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MaterializationMarker {
    pub(crate) schema: String,
    pub(crate) release_id: String,
    pub(crate) source_commit: String,
    pub(crate) snapshot_digest: String,
    pub(crate) snapshot_format: String,
}

pub(crate) fn persist_marker(
    path: &Path,
    marker: &MaterializationMarker,
) -> Result<(), SourceMaterializationPortError> {
    let bytes = serde_json::to_vec_pretty(marker)
        .map_err(|failure| error("source_materialization_marker_encode_failed", failure))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|failure| error("source_materialization_marker_create_failed", failure))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|failure| error("source_materialization_marker_write_failed", failure))
}

pub(crate) fn read_marker(
    path: &Path,
) -> Result<MaterializationMarker, SourceMaterializationPortError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|failure| error("source_materialization_marker_unavailable", failure))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(SourceMaterializationPortError::new(
            "source_materialization_marker_unsafe",
            "marker must be a regular file",
        ));
    }
    let bytes = fs::read(path)
        .map_err(|failure| error("source_materialization_marker_read_failed", failure))?;
    serde_json::from_slice(&bytes)
        .map_err(|failure| error("source_materialization_marker_invalid", failure))
}

fn error(code: &str, failure: impl std::fmt::Display) -> SourceMaterializationPortError {
    SourceMaterializationPortError::new(code, failure.to_string())
}
