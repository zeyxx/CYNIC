use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

<<<<<<< HEAD
use cynic_protocol::{EvidenceEnvelope, build_evidence_envelope_with_lineage};
=======
use cynic_protocol::EvidenceEnvelope;
>>>>>>> github/main

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
<<<<<<< HEAD

    fn read(&self, digest: &str) -> Result<Option<EvidenceEnvelope>, String> {
        let path = self.record_path(digest)?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("observe evidence {}: {error}", path.display())),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("evidence_read_path_rejected".to_string());
        }
        let bytes = fs::read(&path)
            .map_err(|error| format!("read evidence {}: {error}", path.display()))?;
        let envelope = serde_json::from_slice::<EvidenceEnvelope>(&bytes)
            .map_err(|error| format!("decode evidence {}: {error}", path.display()))?;
        let rebuilt = build_evidence_envelope_with_lineage(
            &envelope.subject,
            &envelope.producer,
            envelope.observed_at_unix_ms,
            envelope.provenance.clone(),
            envelope.lineage.clone(),
            &envelope.payload,
        )?;
        if rebuilt != envelope || envelope.digest != digest {
            return Err("evidence_read_back_digest_mismatch".to_string());
        }
        Ok(Some(envelope))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::build_evidence_envelope;

    #[test]
    fn reads_back_only_the_exact_content_addressed_envelope() {
        let root = tempfile::tempdir().unwrap();
        let store = LocalContentAddressedEvidenceStore::new(root.path());
        let envelope = build_evidence_envelope(
            "build:test",
            "builder:test",
            1,
            vec!["test".to_string()],
            &serde_json::json!({"status": "passed"}),
        )
        .unwrap();

        store.persist(&envelope).unwrap();

        assert_eq!(store.read(&envelope.digest).unwrap(), Some(envelope));
        assert!(
            store
                .read(&format!("sha256:{}", "0".repeat(64)))
                .unwrap()
                .is_none()
        );
    }
=======
>>>>>>> github/main
}
