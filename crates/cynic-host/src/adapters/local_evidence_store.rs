use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use cynic_protocol::EvidenceEnvelope;

use crate::ports::evidence_store::ContentAddressedEvidenceStorePort;

#[derive(Debug, Clone)]
pub struct LocalContentAddressedEvidenceStore {
    root: PathBuf,
}

impl LocalContentAddressedEvidenceStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn record_path(&self, digest: &str) -> Result<PathBuf, String> {
        let name = digest
            .strip_prefix("sha256:")
            .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or_else(|| "invalid_evidence_digest".to_string())?;
        Ok(self.root.join(format!("{name}.json")))
    }
}

impl ContentAddressedEvidenceStorePort for LocalContentAddressedEvidenceStore {
    fn persist(&self, envelope: &EvidenceEnvelope) -> Result<String, String> {
        fs::create_dir_all(&self.root)
            .map_err(|error| format!("create evidence store {}: {error}", self.root.display()))?;
        let path = self.record_path(&envelope.digest)?;
        let bytes = serde_json::to_vec_pretty(envelope)
            .map_err(|error| format!("serialize evidence envelope: {error}"))?;
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => file
                .write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|error| format!("persist evidence {}: {error}", path.display()))?,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = fs::read(&path).map_err(|read_error| {
                    format!("read evidence {}: {read_error}", path.display())
                })?;
                if existing != bytes {
                    return Err("evidence_digest_collision_or_record_mismatch".to_string());
                }
            }
            Err(error) => return Err(format!("create evidence {}: {error}", path.display())),
        }
        Ok(path.display().to_string())
    }
}
