use super::{BackfillStore, StorageError, StoredPageReceipt};
use crate::solana_archive::{
    BackfillCheckpoint, BackfillSpec, IndexedHistoryPage, ProviderKind, SourceRole,
};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct FileBackfillStore {
    root: PathBuf,
}

impl FileBackfillStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn target_root(&self, spec: &BackfillSpec) -> PathBuf {
        self.root.join(target_namespace(spec))
    }
}

impl BackfillStore for FileBackfillStore {
    fn persist_page(
        &self,
        spec: &BackfillSpec,
        page_index: u32,
        page: &IndexedHistoryPage,
    ) -> Result<StoredPageReceipt, StorageError> {
        validate_page(spec, page)?;
        let bytes = serde_json::to_vec_pretty(page)
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        let content_digest = format!("blake3:{}", blake3::hash(&bytes).to_hex());
        let digest_suffix = content_digest.trim_start_matches("blake3:");
        let relative_path =
            PathBuf::from("pages").join(format!("{page_index:08}-{}.json", &digest_suffix[..16]));
        let target = self.target_root(spec).join(&relative_path);
        let already_present = write_append_only(&target, &bytes)?;
        Ok(StoredPageReceipt {
            relative_path: relative_path.to_string_lossy().to_string(),
            content_digest,
            already_present,
        })
    }

    fn persist_checkpoint(
        &self,
        spec: &BackfillSpec,
        checkpoint: &BackfillCheckpoint,
    ) -> Result<(), StorageError> {
        let bytes = serde_json::to_vec_pretty(checkpoint)
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        let target = self.target_root(spec).join("checkpoint.json");
        write_atomically(&target, &bytes)
    }

    fn load_checkpoint(
        &self,
        spec: &BackfillSpec,
    ) -> Result<Option<BackfillCheckpoint>, StorageError> {
        let path = self.target_root(spec).join("checkpoint.json");
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| StorageError::Serialization(error.to_string())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(StorageError::Io(error.to_string())),
        }
    }
}

fn validate_page(spec: &BackfillSpec, page: &IndexedHistoryPage) -> Result<(), StorageError> {
    if page.provider != ProviderKind::Helius || page.role != SourceRole::IndexedPrimary {
        return Err(StorageError::InvalidInput(
            "backfill page is not a Helius indexed-primary observation".to_string(),
        ));
    }
    if page.query.cluster != spec.cluster || page.query.target != spec.target {
        return Err(StorageError::InvalidInput(
            "backfill page does not match the declared target".to_string(),
        ));
    }
    if page.response_digest.trim().is_empty() {
        return Err(StorageError::InvalidInput(
            "provider response digest is missing".to_string(),
        ));
    }
    Ok(())
}

fn target_namespace(spec: &BackfillSpec) -> String {
    let identity = format!("{}\0{}", spec.cluster, spec.target);
    format!("target-{}", blake3::hash(identity.as_bytes()).to_hex())
}

fn write_append_only(path: &Path, bytes: &[u8]) -> Result<bool, StorageError> {
    if path.exists() {
        let existing = fs::read(path).map_err(|error| StorageError::Io(error.to_string()))?;
        if existing == bytes {
            return Ok(true);
        }
        return Err(StorageError::IntegrityConflict(
            "append-only page path already contains different bytes".to_string(),
        ));
    }
    write_new_atomically(path, bytes)?;
    Ok(false)
}

fn write_new_atomically(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let parent = path
        .parent()
        .ok_or_else(|| StorageError::InvalidInput("storage path has no parent".to_string()))?;
    fs::create_dir_all(parent).map_err(|error| StorageError::Io(error.to_string()))?;
    let temporary = temporary_path(path);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| StorageError::Io(error.to_string()))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| StorageError::Io(error.to_string()))?;
    fs::rename(&temporary, path).map_err(|error| StorageError::Io(error.to_string()))?;
    sync_directory(parent)?;
    Ok(())
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let parent = path
        .parent()
        .ok_or_else(|| StorageError::InvalidInput("storage path has no parent".to_string()))?;
    fs::create_dir_all(parent).map_err(|error| StorageError::Io(error.to_string()))?;
    let temporary = temporary_path(path);
    let mut file = File::create(&temporary).map_err(|error| StorageError::Io(error.to_string()))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| StorageError::Io(error.to_string()))?;
    fs::rename(&temporary, path).map_err(|error| StorageError::Io(error.to_string()))?;
    sync_directory(parent)?;
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    path.with_extension(format!("tmp-{}-{nonce}", std::process::id()))
}

fn sync_directory(path: &Path) -> Result<(), StorageError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| StorageError::Io(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::{Commitment, HistoryFilters, SortOrder};

    fn spec(target: &str) -> BackfillSpec {
        BackfillSpec {
            cluster: "mainnet".to_string(),
            target: target.to_string(),
            page_limit: 100,
            max_pages: 1,
            max_records: 100,
            sort_order: SortOrder::Ascending,
            commitment: Commitment::Finalized,
            filters: HistoryFilters::default(),
        }
    }

    #[test]
    fn target_namespace_is_stable_and_target_scoped() {
        assert_eq!(target_namespace(&spec("a")), target_namespace(&spec("a")));
        assert_ne!(target_namespace(&spec("a")), target_namespace(&spec("b")));
    }
}
