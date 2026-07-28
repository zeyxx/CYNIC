use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use sha2::{Digest, Sha256};

use crate::ports::{
    agent_workspace_port::AgentWorkspacePortError,
    source_change_transaction_port::AgentSourceBlobStorePort,
};

#[derive(Debug, Clone)]
pub struct LocalAgentSourceBlobStore {
    root: PathBuf,
}

impl LocalAgentSourceBlobStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn blob_path(&self, digest: &str) -> Result<PathBuf, AgentWorkspacePortError> {
        let name = digest
            .strip_prefix("sha256:")
            .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or_else(|| {
                AgentWorkspacePortError::new(
                    "source_blob_digest_invalid",
                    "source blob digest must be sha256",
                )
            })?;
        Ok(self.root.join(name))
    }
}

impl AgentSourceBlobStorePort for LocalAgentSourceBlobStore {
    fn persist(
        &self,
        expected_digest: &str,
        content: &[u8],
    ) -> Result<(), AgentWorkspacePortError> {
        verify_content_digest(expected_digest, content)?;
        fs::create_dir_all(&self.root).map_err(|error| {
            AgentWorkspacePortError::new(
                "source_blob_store_unavailable",
                format!("cannot create source blob store: {error}"),
            )
        })?;
        let path = self.blob_path(expected_digest)?;
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => file
                .write_all(content)
                .and_then(|_| file.sync_all())
                .map_err(|error| {
                    AgentWorkspacePortError::new(
                        "source_blob_persist_failed",
                        format!("cannot persist source blob: {error}"),
                    )
                }),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = fs::read(&path).map_err(|read_error| {
                    AgentWorkspacePortError::new(
                        "source_blob_read_failed",
                        format!("cannot verify existing source blob: {read_error}"),
                    )
                })?;
                verify_content_digest(expected_digest, &existing)?;
                if existing != content {
                    return Err(AgentWorkspacePortError::new(
                        "source_blob_collision",
                        "existing source blob differs from identical digest input",
                    ));
                }
                Ok(())
            }
            Err(error) => Err(AgentWorkspacePortError::new(
                "source_blob_persist_failed",
                format!("cannot create source blob: {error}"),
            )),
        }
    }

    fn load(&self, digest: &str) -> Result<Vec<u8>, AgentWorkspacePortError> {
        let content = fs::read(self.blob_path(digest)?).map_err(|error| {
            AgentWorkspacePortError::new(
                "source_blob_unavailable",
                format!("cannot load source blob: {error}"),
            )
        })?;
        verify_content_digest(digest, &content)?;
        Ok(content)
    }
}

fn verify_content_digest(
    expected_digest: &str,
    content: &[u8],
) -> Result<(), AgentWorkspacePortError> {
    let observed = format!("sha256:{}", hex::encode(Sha256::digest(content)));
    if observed != expected_digest {
        return Err(AgentWorkspacePortError::new(
            "source_blob_digest_mismatch",
            format!("source blob hash '{observed}' differs from expected digest"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(content: &[u8]) -> String {
        format!("sha256:{}", hex::encode(Sha256::digest(content)))
    }

    #[test]
    fn persists_and_reads_exact_content_addressed_blob() {
        let root = tempfile::tempdir().unwrap();
        let store = LocalAgentSourceBlobStore::new(root.path());
        let content = b"bounded change\n";
        let digest = digest(content);

        store.persist(&digest, content).unwrap();

        assert_eq!(store.load(&digest).unwrap(), content);
    }

    #[test]
    fn rejects_content_that_does_not_match_declared_digest() {
        let root = tempfile::tempdir().unwrap();
        let store = LocalAgentSourceBlobStore::new(root.path());
        assert!(store.persist(&digest(b"expected"), b"drift").is_err());
    }
}
