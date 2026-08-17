use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::PathBuf;

use cynic_protocol::{
    CausalEpochIndex, CausalEpochIndexAppend, CausalEpochIndexEntry, CausalEpochKey, digest_json,
};

use crate::ports::causal_epoch_index::CausalEpochIndexStorePort;

#[derive(Debug, Clone)]
pub struct LocalCausalEpochIndexStore {
    root: PathBuf,
}

impl LocalCausalEpochIndexStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn record_path(&self, entry: &CausalEpochIndexEntry) -> Result<PathBuf, String> {
        let digest =
            digest_json(entry).map_err(|error| format!("digest epoch index entry: {error}"))?;
        let name = digest
            .strip_prefix("sha256:")
            .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or_else(|| "invalid_epoch_index_entry_digest".to_string())?;
        Ok(self.root.join(format!("{name}.json")))
    }

    fn read_entry(&self, path: PathBuf) -> Result<CausalEpochIndexEntry, String> {
        let bytes = fs::read(&path)
            .map_err(|error| format!("read epoch index entry {}: {error}", path.display()))?;
        let entry = serde_json::from_slice::<CausalEpochIndexEntry>(&bytes)
            .map_err(|error| format!("decode epoch index entry {}: {error}", path.display()))?;
        let expected_digest = digest_json(&entry)
            .map_err(|error| format!("digest epoch index read-back entry: {error}"))?;
        let expected_name = expected_digest
            .strip_prefix("sha256:")
            .ok_or_else(|| "epoch_index_read_back_digest_invalid".to_string())?;
        if path.file_stem().and_then(|value| value.to_str()) != Some(expected_name) {
            return Err("epoch_index_entry_digest_read_back_mismatch".to_string());
        }
        Ok(entry)
    }

    fn persist_entry(&self, entry: &CausalEpochIndexEntry) -> Result<(), String> {
        fs::create_dir_all(&self.root)
            .map_err(|error| format!("create epoch index {}: {error}", self.root.display()))?;
        let path = self.record_path(entry)?;
        let bytes = serde_json::to_vec_pretty(entry)
            .map_err(|error| format!("serialize epoch index entry: {error}"))?;
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => file
                .write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|error| format!("persist epoch index {}: {error}", path.display()))?,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = fs::read(&path).map_err(|read_error| {
                    format!("read epoch index {}: {read_error}", path.display())
                })?;
                if existing != bytes {
                    return Err("epoch_index_digest_collision_or_record_mismatch".to_string());
                }
            }
            Err(error) => return Err(format!("create epoch index {}: {error}", path.display())),
        }
        Ok(())
    }

    fn lock(&self) -> Result<EpochIndexLock, String> {
        fs::create_dir_all(&self.root)
            .map_err(|error| format!("create epoch index {}: {error}", self.root.display()))?;
        let path = self.root.join(".index.lock");
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("open epoch index lock {}: {error}", path.display()))?;
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
        if result != 0 {
            return Err(format!(
                "lock epoch index {}: {}",
                path.display(),
                std::io::Error::last_os_error()
            ));
        }
        Ok(EpochIndexLock(file))
    }
}

struct EpochIndexLock(std::fs::File);

impl Drop for EpochIndexLock {
    fn drop(&mut self) {
        let _ = unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

impl CausalEpochIndexStorePort for LocalCausalEpochIndexStore {
    fn append(&self, entry: &CausalEpochIndexEntry) -> Result<CausalEpochIndexAppend, String> {
        let _lock = self.lock()?;
        let mut current = self.read_index()?;
        let outcome = current.append(entry.clone())?;
        if outcome == CausalEpochIndexAppend::AlreadyPresent {
            return Ok(outcome);
        }

        self.persist_entry(entry)?;
        let read_back = self.read_index()?;
        match read_back.get(&entry.key) {
            Some(candidate) if candidate == entry => Ok(outcome),
            Some(_) => Err("epoch_index_coordinate_read_back_conflict".to_string()),
            None => Err("epoch_index_entry_read_back_missing".to_string()),
        }
    }

    fn read_index(&self) -> Result<CausalEpochIndex, String> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(CausalEpochIndex::new());
            }
            Err(error) => {
                return Err(format!("read epoch index {}: {error}", self.root.display()));
            }
        };
        let mut records = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|error| format!("enumerate epoch index {}: {error}", self.root.display()))?
                .path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            records.push(self.read_entry(path)?);
        }
        CausalEpochIndex::from_entries(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::causal_epoch_index::CausalEpochIndexStorePort;

    fn entry(epoch_id: &str, marker: char, observed_at_unix_ms: u128) -> CausalEpochIndexEntry {
        CausalEpochIndexEntry {
            key: CausalEpochKey {
                subject_id: "local-kernel".to_string(),
                vertical_id: "process".to_string(),
                epoch_id: epoch_id.to_string(),
            },
            epoch_digest: format!("sha256:{}", marker.to_string().repeat(64)),
            observed_at_unix_ms,
        }
    }

    #[test]
    fn local_index_is_create_once_and_read_back_exactly() {
        let root = tempfile::tempdir().unwrap();
        let store = LocalCausalEpochIndexStore::new(root.path());
        let first = entry("epoch-1", '1', 1);
        let second = entry("epoch-2", '2', 2);

        assert_eq!(
            store.append(&first).unwrap(),
            CausalEpochIndexAppend::Inserted
        );
        assert_eq!(
            store.append(&first).unwrap(),
            CausalEpochIndexAppend::AlreadyPresent
        );
        store.append(&second).unwrap();

        let reloaded = LocalCausalEpochIndexStore::new(root.path());
        assert_eq!(reloaded.read_index().unwrap().entries.len(), 2);
        assert_eq!(
            reloaded
                .latest_for("local-kernel", "process")
                .unwrap()
                .unwrap()
                .key
                .epoch_id,
            "epoch-2"
        );
    }

    #[test]
    fn coordinate_conflict_survives_adapter_boundary() {
        let root = tempfile::tempdir().unwrap();
        let store = LocalCausalEpochIndexStore::new(root.path());
        store.append(&entry("epoch-1", '1', 1)).unwrap();

        let error = store.append(&entry("epoch-1", '2', 1)).unwrap_err();
        assert_eq!(error, "causal_epoch_index_coordinate_conflict");
    }
}
