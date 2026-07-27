use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;

use super::types::CONTENT_ENVELOPE_SCHEMA;
use super::{ContentEnvelope, ContentReference, ContentVaultPort};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalContentRegistration {
    pub content_ref: ContentReference,
    pub relative_path: PathBuf,
    pub provenance: String,
    pub retention: String,
}

#[derive(Debug)]
pub struct LocalContentVault {
    canonical_root: PathBuf,
    registrations: BTreeMap<ContentReference, LocalContentRegistration>,
}

#[derive(Debug, Error)]
pub enum LocalContentVaultError {
    #[error("vault root is unavailable: {0}")]
    RootUnavailable(String),
    #[error("content registration is invalid: {0}")]
    InvalidRegistration(String),
    #[error("content reference is unknown: {0}")]
    UnknownReference(String),
    #[error("content path cannot be observed: {0}")]
    PathUnavailable(String),
    #[error("content path escapes the vault root: {0}")]
    PathEscape(String),
    #[error("content cannot be read: {0}")]
    ReadFailed(String),
}

impl LocalContentVault {
    pub fn new(
        root: impl AsRef<Path>,
        registrations: Vec<LocalContentRegistration>,
    ) -> Result<Self, LocalContentVaultError> {
        let canonical_root = root
            .as_ref()
            .canonicalize()
            .map_err(|error| LocalContentVaultError::RootUnavailable(error.to_string()))?;
        if !canonical_root.is_dir() {
            return Err(LocalContentVaultError::RootUnavailable(
                "root is not a directory".to_string(),
            ));
        }

        let mut ids = BTreeSet::new();
        let mut indexed = BTreeMap::new();
        for registration in registrations {
            validate_registration(&registration)?;
            if !ids.insert(registration.content_ref.id.clone()) {
                return Err(LocalContentVaultError::InvalidRegistration(format!(
                    "duplicate reference {}",
                    registration.content_ref.id
                )));
            }
            indexed.insert(registration.content_ref.clone(), registration);
        }

        Ok(Self {
            canonical_root,
            registrations: indexed,
        })
    }
}

impl ContentVaultPort for LocalContentVault {
    type Error = LocalContentVaultError;

    fn observe_envelope(
        &self,
        content_ref: &ContentReference,
    ) -> Result<ContentEnvelope, Self::Error> {
        let registration = self
            .registrations
            .get(content_ref)
            .ok_or_else(|| LocalContentVaultError::UnknownReference(content_ref.id.clone()))?;
        let candidate = self.canonical_root.join(&registration.relative_path);
        let canonical_path = candidate
            .canonicalize()
            .map_err(|error| LocalContentVaultError::PathUnavailable(error.to_string()))?;
        if !canonical_path.starts_with(&self.canonical_root) {
            return Err(LocalContentVaultError::PathEscape(content_ref.id.clone()));
        }

        let mut file = File::open(&canonical_path)
            .map_err(|error| LocalContentVaultError::ReadFailed(error.to_string()))?;
        let metadata = file
            .metadata()
            .map_err(|error| LocalContentVaultError::ReadFailed(error.to_string()))?;
        if !metadata.is_file() {
            return Err(LocalContentVaultError::ReadFailed(
                "registered content is not a regular file".to_string(),
            ));
        }

        let mut hasher = Sha256::new();
        let mut prefix = Vec::with_capacity(16);
        let mut buffer = [0_u8; 16 * 1024];
        let mut observed_size = 0_u64;
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|error| LocalContentVaultError::ReadFailed(error.to_string()))?;
            if read == 0 {
                break;
            }
            observed_size = observed_size.saturating_add(read as u64);
            let prefix_remaining = 16_usize.saturating_sub(prefix.len());
            prefix.extend_from_slice(&buffer[..read.min(prefix_remaining)]);
            hasher.update(&buffer[..read]);
        }
        let digest = format!("sha256:{:x}", hasher.finalize());
        let mut residual_unknowns = vec![
            "content_semantics_not_observed".to_string(),
            "filesystem_path_race_hardening_not_proved".to_string(),
        ];
        if metadata.len() != observed_size {
            residual_unknowns.push("content_changed_while_observed".to_string());
        }

        Ok(ContentEnvelope {
            schema: CONTENT_ENVELOPE_SCHEMA.to_string(),
            content_ref: content_ref.clone(),
            media_type: detect_media_type(&prefix).to_string(),
            size_bytes: observed_size,
            digest: digest.clone(),
            provenance: registration.provenance.clone(),
            retention: registration.retention.clone(),
            raw_content_in_report: false,
            evidence_refs: vec![digest],
            residual_unknowns,
        })
    }
}

fn validate_registration(
    registration: &LocalContentRegistration,
) -> Result<(), LocalContentVaultError> {
    if registration.content_ref.id.trim().is_empty() {
        return Err(LocalContentVaultError::InvalidRegistration(
            "reference id is empty".to_string(),
        ));
    }
    if registration.relative_path.is_absolute()
        || registration
            .relative_path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(LocalContentVaultError::InvalidRegistration(
            "path must be relative and cannot contain parent traversal".to_string(),
        ));
    }
    if registration.provenance.trim().is_empty() || registration.retention.trim().is_empty() {
        return Err(LocalContentVaultError::InvalidRegistration(
            "provenance and retention are required".to_string(),
        ));
    }
    Ok(())
}

fn detect_media_type(prefix: &[u8]) -> &'static str {
    if prefix.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if prefix.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if prefix.starts_with(b"GIF87a") || prefix.starts_with(b"GIF89a") {
        "image/gif"
    } else if prefix.len() >= 12 && &prefix[..4] == b"RIFF" && &prefix[8..12] == b"WEBP" {
        "image/webp"
    } else if prefix.len() >= 8 && &prefix[4..8] == b"ftyp" {
        "video/mp4"
    } else {
        "application/octet-stream"
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    fn registration(path: &str) -> LocalContentRegistration {
        LocalContentRegistration {
            content_ref: ContentReference {
                id: "incoming:alice:1".to_string(),
            },
            relative_path: PathBuf::from(path),
            provenance: "local_message_adapter".to_string(),
            retention: "until_alice_decides".to_string(),
        }
    }

    #[test]
    fn observes_a_non_disclosing_envelope() {
        let root = tempdir().expect("vault");
        let payload = b"\x89PNG\r\n\x1a\nsynthetic-fixture";
        fs::write(root.path().join("incoming.bin"), payload).expect("fixture");
        let vault =
            LocalContentVault::new(root.path(), vec![registration("incoming.bin")]).expect("vault");

        let envelope = vault
            .observe_envelope(&ContentReference {
                id: "incoming:alice:1".to_string(),
            })
            .expect("envelope");

        assert_eq!(envelope.media_type, "image/png");
        assert_eq!(envelope.size_bytes, payload.len() as u64);
        assert!(!envelope.raw_content_in_report);
        let serialized = serde_json::to_string(&envelope).expect("serialize");
        assert!(!serialized.contains("synthetic-fixture"));
        assert!(!serialized.contains(root.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn rejects_parent_traversal_at_registration() {
        let root = tempdir().expect("vault");

        let error = LocalContentVault::new(root.path(), vec![registration("../outside")])
            .expect_err("traversal must fail");

        assert!(matches!(
            error,
            LocalContentVaultError::InvalidRegistration(_)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlink_that_resolves_outside_the_vault() {
        use std::os::unix::fs::symlink;

        let root = tempdir().expect("vault");
        let outside = tempfile::NamedTempFile::new().expect("outside");
        symlink(outside.path(), root.path().join("escape")).expect("symlink");
        let vault =
            LocalContentVault::new(root.path(), vec![registration("escape")]).expect("vault");

        let error = vault
            .observe_envelope(&ContentReference {
                id: "incoming:alice:1".to_string(),
            })
            .expect_err("escape must fail");

        assert!(matches!(error, LocalContentVaultError::PathEscape(_)));
    }
}
