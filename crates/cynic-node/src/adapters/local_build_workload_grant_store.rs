use std::{
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use cynic_build::{SignedBuildWorkloadGrant, validate_signed_build_workload_grant};

use crate::ports::build_workload_grant_store_port::{
    BuildWorkloadGrantStoreError, BuildWorkloadGrantStorePort, PersistedBuildWorkloadGrant,
};

#[derive(Debug, Clone)]
pub struct LocalBuildWorkloadGrantStore {
    root: PathBuf,
}

impl LocalBuildWorkloadGrantStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn workload_root(&self, workload_id: &str) -> Result<PathBuf, BuildWorkloadGrantStoreError> {
        validate_id(workload_id)?;
        Ok(self.root.join(workload_id))
    }
}

impl BuildWorkloadGrantStorePort for LocalBuildWorkloadGrantStore {
    fn latest(
        &self,
        workload_id: &str,
    ) -> Result<Option<PersistedBuildWorkloadGrant>, BuildWorkloadGrantStoreError> {
        let root = self.workload_root(workload_id)?;
        let entries = match fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(failure) if failure.kind() == ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(error("build_grant_store_root_unobservable")),
        };
        reject_directory(&root)?;
        let mut paths = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|_| error("build_grant_store_entry_unobservable"))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with("issued-") && name.ends_with(".json") {
                paths.push(entry.path());
            }
        }
        paths.sort();
        paths
            .last()
            .map(|path| read_grant(path, workload_id))
            .transpose()
    }

    fn persist(
        &self,
        grant: &SignedBuildWorkloadGrant,
    ) -> Result<PersistedBuildWorkloadGrant, BuildWorkloadGrantStoreError> {
        if !validate_signed_build_workload_grant(grant).is_empty() {
            return Err(error("build_grant_store_record_rejected"));
        }
        let workload_id = &grant.grant.workload_id;
        let root = self.workload_root(workload_id)?;
        fs::create_dir_all(&self.root)
            .map_err(|_| error("build_grant_store_root_create_failed"))?;
        reject_directory(&self.root)?;
        fs::create_dir(&root)
            .or_else(|failure| {
                if failure.kind() == ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(failure)
                }
            })
            .map_err(|_| error("build_grant_store_workload_root_create_failed"))?;
        reject_directory(&root)?;

        let path = root.join(format!(
            "issued-{:020}-revision-{:020}-fence-{:020}.json",
            grant.grant.issued_at_unix_s, grant.grant.workload_revision, grant.grant.fencing_token
        ));
        let bytes = serde_json::to_vec_pretty(grant)
            .map_err(|_| error("build_grant_store_encode_failed"))?;
        match write_new(&path, &bytes) {
            Ok(()) => {
                File::open(&root)
                    .and_then(|directory| directory.sync_all())
                    .map_err(|_| error("build_grant_store_directory_sync_failed"))?;
            }
            Err(failure) if failure.code == "build_grant_store_record_exists" => {
                let observed = read_grant(&path, workload_id)?;
                if observed.grant != *grant {
                    return Err(error("build_grant_store_immutable_conflict"));
                }
                return Ok(observed);
            }
            Err(failure) => return Err(failure),
        }
        let observed = read_grant(&path, workload_id)?;
        if observed.grant != *grant {
            return Err(error("build_grant_store_read_back_mismatch"));
        }
        Ok(observed)
    }
}

fn read_grant(
    path: &Path,
    workload_id: &str,
) -> Result<PersistedBuildWorkloadGrant, BuildWorkloadGrantStoreError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| error("build_grant_store_record_unobservable"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(error("build_grant_store_record_path_rejected"));
    }
    let bytes = fs::read(path).map_err(|_| error("build_grant_store_record_read_failed"))?;
    let grant: SignedBuildWorkloadGrant =
        serde_json::from_slice(&bytes).map_err(|_| error("build_grant_store_record_invalid"))?;
    if grant.grant.workload_id != workload_id
        || !validate_signed_build_workload_grant(&grant).is_empty()
    {
        return Err(error("build_grant_store_record_rejected"));
    }
    Ok(PersistedBuildWorkloadGrant {
        grant,
        evidence_ref: format!("file:build-grants/{workload_id}/{}", file_name(path)?),
        path: path.to_path_buf(),
    })
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), BuildWorkloadGrantStoreError> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o640);
    let mut file = options.open(path).map_err(|failure| {
        if failure.kind() == ErrorKind::AlreadyExists {
            error("build_grant_store_record_exists")
        } else {
            error("build_grant_store_record_create_failed")
        }
    })?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| error("build_grant_store_record_write_failed"))
}

fn reject_directory(path: &Path) -> Result<(), BuildWorkloadGrantStoreError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| error("build_grant_store_root_unobservable"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(error("build_grant_store_root_rejected"));
    }
    Ok(())
}

fn validate_id(value: &str) -> Result<(), BuildWorkloadGrantStoreError> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(error("build_grant_store_identifier_invalid"));
    }
    Ok(())
}

fn file_name(path: &Path) -> Result<&str, BuildWorkloadGrantStoreError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| error("build_grant_store_record_name_invalid"))
}

fn error(code: &str) -> BuildWorkloadGrantStoreError {
    BuildWorkloadGrantStoreError::new(code, code)
}

#[cfg(test)]
mod tests {
    use cynic_build::{
        BUILD_WORKLOAD_GRANT_SCHEMA, BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE, BuildWorkloadGrant,
        SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA,
    };

    use super::*;

    fn signed_grant() -> SignedBuildWorkloadGrant {
        SignedBuildWorkloadGrant {
            schema: SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA.to_string(),
            authority_key_id: format!("sha256:{}", "a".repeat(64)),
            signature_namespace: BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE.to_string(),
            grant: BuildWorkloadGrant {
                schema: BUILD_WORKLOAD_GRANT_SCHEMA.to_string(),
                status: "admitted".to_string(),
                grant_id: "build-workload:workload-1:2:7".to_string(),
                issuer_node_id: "scheduler-1".to_string(),
                workload_id: "workload-1".to_string(),
                workload_revision: 2,
                workload_record_digest: format!("sha256:{}", "b".repeat(64)),
                node_id: "builder-1".to_string(),
                role: "build_runner".to_string(),
                source_commit: "c".repeat(40),
                policy_digest: format!("sha256:{}", "d".repeat(64)),
                lease_id: "lease-1".to_string(),
                fencing_token: 7,
                cancellation_generation: 0,
                issued_at_unix_s: 100,
                expires_at_unix_s: 200,
                allowed_actions: vec!["make:check".to_string()],
                evidence_refs: vec!["workload-record:proof".to_string()],
            },
            signature_hex: "e".repeat(128),
        }
    }

    #[test]
    fn persisted_grant_is_idempotent_and_read_back() {
        let root = tempfile::tempdir().expect("temporary root");
        let store = LocalBuildWorkloadGrantStore::new(root.path());
        let grant = signed_grant();

        let first = store.persist(&grant).expect("first persist");
        let second = store.persist(&grant).expect("idempotent persist");
        let latest = store
            .latest("workload-1")
            .expect("latest read")
            .expect("persisted grant");

        assert_eq!(first, second);
        assert_eq!(latest, first);
    }

    #[test]
    fn immutable_grant_identity_cannot_be_overwritten() {
        let root = tempfile::tempdir().expect("temporary root");
        let store = LocalBuildWorkloadGrantStore::new(root.path());
        let grant = signed_grant();
        store.persist(&grant).expect("first persist");

        let mut conflicting = grant;
        conflicting.signature_hex = "f".repeat(128);
        let failure = store.persist(&conflicting).expect_err("immutable conflict");

        assert_eq!(failure.code, "build_grant_store_immutable_conflict");
    }
}
