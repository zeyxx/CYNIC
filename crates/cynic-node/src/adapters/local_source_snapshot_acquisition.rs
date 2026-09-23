use std::path::PathBuf;

use cynic_vcs::SOURCE_MATERIALIZATION_FORMAT;

use crate::{
    adapters::source_snapshot_store::{snapshot_path, verify_snapshot},
    ports::source_materialization_port::{
        SourceMaterializationPortError, SourceSnapshotAcquisitionPort,
    },
};

#[derive(Debug)]
pub struct LocalSourceSnapshotAcquisition {
    snapshot_root: PathBuf,
}

impl LocalSourceSnapshotAcquisition {
    pub fn new(snapshot_root: impl Into<PathBuf>) -> Self {
        Self {
            snapshot_root: snapshot_root.into(),
        }
    }
}

impl SourceSnapshotAcquisitionPort for LocalSourceSnapshotAcquisition {
    fn acquire(
        &self,
        snapshot_digest: &str,
        snapshot_format: &str,
    ) -> Result<(), SourceMaterializationPortError> {
        if snapshot_format != SOURCE_MATERIALIZATION_FORMAT {
            return Err(SourceMaterializationPortError::new(
                "source_snapshot_acquisition_format_unsupported",
                "only git_bundle_v1 is supported",
            ));
        }
        let path = snapshot_path(&self.snapshot_root, snapshot_digest)?;
        if !path.exists() {
            return Err(SourceMaterializationPortError::new(
                "source_snapshot_transport_unavailable",
                "snapshot is absent from the node-local content-addressed store",
            ));
        }
        verify_snapshot(&path, snapshot_digest)
    }
}
