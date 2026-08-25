use std::fs;
use std::path::{Path, PathBuf};

use super::types::{SSH_TRUST_STORE_SCHEMA, SshTrustAnchor, SshTrustStore};

pub const DEFAULT_SSH_TRUST_STORE_PATH: &str = "/etc/cynic/trust/ssh-hosts.json";

pub fn default_ssh_trust_store_path() -> PathBuf {
    std::env::var_os("CYNIC_SSH_TRUST_STORE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SSH_TRUST_STORE_PATH))
}

pub fn load_ssh_trust_store(path: &Path) -> Result<SshTrustStore, String> {
    match fs::read(path) {
        Ok(bytes) => {
            let store: SshTrustStore = serde_json::from_slice(&bytes)
                .map_err(|error| format!("parse SSH trust store {}: {error}", path.display()))?;
            if store.schema != SSH_TRUST_STORE_SCHEMA {
                return Err(format!(
                    "unsupported SSH trust store schema: {}",
                    store.schema
                ));
            }
            Ok(store)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(SshTrustStore::empty()),
        Err(error) => Err(format!("read SSH trust store {}: {error}", path.display())),
    }
}

pub fn admit_ssh_trust_anchor(path: &Path, anchor: SshTrustAnchor) -> Result<bool, String> {
    let mut store = load_ssh_trust_store(path)?;
    if let Some(existing) = store
        .anchors
        .iter()
        .find(|existing| existing.target == anchor.target)
    {
        if existing.algorithm == anchor.algorithm && existing.fingerprint == anchor.fingerprint {
            return Ok(false);
        }
        return Err(format!(
            "ssh_trust_anchor_rotation_requires_explicit_replacement:{}",
            anchor.target
        ));
    }
    store.anchors.push(anchor);
    store
        .anchors
        .sort_by(|left, right| left.target.cmp(&right.target));
    persist_ssh_trust_store(path, &store)?;
    Ok(true)
}

fn persist_ssh_trust_store(path: &Path, store: &SshTrustStore) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("SSH trust store has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "create SSH trust store directory {}: {error}",
            parent.display()
        )
    })?;
    let bytes = serde_json::to_vec_pretty(store)
        .map_err(|error| format!("serialize SSH trust store: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes)
        .map_err(|error| format!("write SSH trust store {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("activate SSH trust store {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor(fingerprint: &str) -> SshTrustAnchor {
        SshTrustAnchor {
            target: "10.10.10.1".to_string(),
            algorithm: "ED25519".to_string(),
            fingerprint: fingerprint.to_string(),
            attestation_source: "console".to_string(),
            admitted_at_unix_ms: 1,
        }
    }

    #[test]
    fn admission_persists_once_and_refuses_silent_rotation() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("ssh-hosts.json");
        assert!(admit_ssh_trust_anchor(&path, anchor("SHA256:first")).expect("admit"));
        assert!(!admit_ssh_trust_anchor(&path, anchor("SHA256:first")).expect("idempotent"));
        assert!(
            admit_ssh_trust_anchor(&path, anchor("SHA256:rotated"))
                .expect_err("rotation must be explicit")
                .contains("rotation_requires_explicit_replacement")
        );
    }
}
