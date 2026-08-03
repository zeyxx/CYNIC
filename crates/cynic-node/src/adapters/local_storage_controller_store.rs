#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::{fs, path::PathBuf};

use cynic_protocol::{
    STORAGE_CONTROLLER_RECORD_SCHEMA, StorageControllerRecord, digest_json,
    validate_storage_controller_record,
};

use crate::ports::storage_controller_store_port::{
    StorageControllerStoreError, StorageControllerStorePort,
};

#[derive(Debug, Clone)]
pub struct LocalStorageControllerStore {
    root: PathBuf,
}

impl LocalStorageControllerStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    fn resource_root(&self, id: &str) -> Result<PathBuf, StorageControllerStoreError> {
        if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains('\0') {
            return Err(error("storage_controller_resource_id_invalid"));
        }
        Ok(self.root.join(id))
    }
}

pub fn seal_storage_controller_record(record: &mut StorageControllerRecord) -> Result<(), String> {
    record.record_digest.clear();
    record.record_digest =
        digest_json(record).map_err(|_| "storage_controller_store_digest_failed".to_string())?;
    Ok(())
}

impl StorageControllerStorePort for LocalStorageControllerStore {
    fn read_latest(
        &self,
        resource_id: &str,
    ) -> Result<Option<StorageControllerRecord>, StorageControllerStoreError> {
        let root = self.resource_root(resource_id)?;
        let mut revisions = match fs::read_dir(&root) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| name.starts_with("revision-") && name.ends_with(".json"))
                .collect::<Vec<_>>(),
            Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(error("storage_controller_store_root_unobservable")),
        };
        revisions.sort();
        let Some(name) = revisions.last() else {
            return Ok(None);
        };
        let bytes =
            fs::read(root.join(name)).map_err(|_| error("storage_controller_store_read_failed"))?;
        let record: StorageControllerRecord = serde_json::from_slice(&bytes)
            .map_err(|_| error("storage_controller_store_decode_failed"))?;
        validate_record(&record)?;
        Ok(Some(record))
    }

    fn append(
        &self,
        expected_generation: Option<u64>,
        record: &StorageControllerRecord,
    ) -> Result<StorageControllerRecord, StorageControllerStoreError> {
        validate_record(record)?;
        let root = self.resource_root(&record.metadata.id)?;
        fs::create_dir_all(&root)
            .map_err(|_| error("storage_controller_store_root_create_failed"))?;
        let _lock = acquire_lock(&root.join(".lock"))?;
        let previous = self.read_latest(&record.metadata.id)?;
        if previous.as_ref().map(|item| item.metadata.generation) != expected_generation {
            return Err(error("storage_controller_store_generation_conflict"));
        }
        if let Some(previous) = &previous {
            if record.metadata.generation <= previous.metadata.generation {
                return Err(error("storage_controller_store_generation_not_monotonic"));
            }
            if record.previous_record_digest.as_deref() != Some(previous.record_digest.as_str()) {
                return Err(error("storage_controller_store_chain_mismatch"));
            }
        }
        let revision = format!("revision-{:020}.json", record.metadata.generation);
        let temporary = root.join(format!(".{revision}.tmp"));
        let bytes = serde_json::to_vec_pretty(record)
            .map_err(|_| error("storage_controller_store_encode_failed"))?;
        fs::write(&temporary, bytes).map_err(|_| error("storage_controller_store_write_failed"))?;
        fs::rename(&temporary, root.join(&revision))
            .map_err(|_| error("storage_controller_store_publish_failed"))?;
        let read_back = self
            .read_latest(&record.metadata.id)?
            .ok_or_else(|| error("storage_controller_store_read_back_missing"))?;
        if read_back != *record {
            return Err(error("storage_controller_store_read_back_mismatch"));
        }
        Ok(read_back)
    }
}

fn validate_record(record: &StorageControllerRecord) -> Result<(), StorageControllerStoreError> {
    if record.schema != STORAGE_CONTROLLER_RECORD_SCHEMA
        || !validate_storage_controller_record(record).is_empty()
    {
        return Err(error("storage_controller_store_record_rejected"));
    }
    let mut unsigned = record.clone();
    unsigned.record_digest.clear();
    let digest =
        digest_json(&unsigned).map_err(|_| error("storage_controller_store_digest_failed"))?;
    if digest != record.record_digest {
        return Err(error("storage_controller_store_digest_mismatch"));
    }
    Ok(())
}

fn error(code: &str) -> StorageControllerStoreError {
    StorageControllerStoreError {
        code: code.to_string(),
    }
}

fn acquire_lock(path: &std::path::Path) -> Result<std::fs::File, StorageControllerStoreError> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| error("storage_controller_store_lock_open_failed"))?;
    #[cfg(unix)]
    {
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(error("storage_controller_store_lock_failed"));
        }
    }
    Ok(file)
}
