use super::{FanoutReadStore, FanoutStore, StorageError, StoredPageReceipt};
use crate::solana_archive::{
    FanoutCheckpoint, FanoutSpec, ProviderKind, RawTransactionObservation, SignaturePage,
    SourceRole,
};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct FileFanoutStore {
    root: PathBuf,
}

impl FileFanoutStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn target_root(&self, spec: &FanoutSpec) -> PathBuf {
        self.root.join(target_namespace(spec))
    }
}

impl FanoutStore for FileFanoutStore {
    fn persist_signature_page(
        &self,
        spec: &FanoutSpec,
        page_index: u32,
        page: &SignaturePage,
    ) -> Result<StoredPageReceipt, StorageError> {
        validate_page(spec, page)?;
        let bytes = serde_json::to_vec_pretty(page)
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        let digest = content_digest(&bytes);
        let relative_path = PathBuf::from("signature-pages")
            .join(format!("{page_index:08}-{}.json", &digest[7..23]));
        let already_present =
            write_append_only(&self.target_root(spec).join(&relative_path), &bytes)?;
        Ok(StoredPageReceipt {
            relative_path: relative_path.to_string_lossy().to_string(),
            content_digest: digest,
            already_present,
        })
    }

    fn persist_transaction(
        &self,
        spec: &FanoutSpec,
        transaction: &RawTransactionObservation,
    ) -> Result<StoredPageReceipt, StorageError> {
        validate_transaction(spec, transaction)?;
        let bytes = serde_json::to_vec_pretty(transaction)
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        let digest = content_digest(&bytes);
        let signature_key = blake3::hash(transaction.query.signature.as_bytes()).to_hex();
        let relative_path = PathBuf::from("transactions").join(format!("{signature_key}.json"));
        let already_present =
            write_append_only(&self.target_root(spec).join(&relative_path), &bytes)?;
        Ok(StoredPageReceipt {
            relative_path: relative_path.to_string_lossy().to_string(),
            content_digest: digest,
            already_present,
        })
    }

    fn persist_checkpoint(
        &self,
        spec: &FanoutSpec,
        checkpoint: &FanoutCheckpoint,
    ) -> Result<(), StorageError> {
        let bytes = serde_json::to_vec_pretty(checkpoint)
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        write_atomically(
            &self.target_root(spec).join("fanout-checkpoint.json"),
            &bytes,
        )
    }

    fn load_checkpoint(&self, spec: &FanoutSpec) -> Result<Option<FanoutCheckpoint>, StorageError> {
        match fs::read(self.target_root(spec).join("fanout-checkpoint.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| StorageError::Serialization(error.to_string())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(StorageError::Io(error.to_string())),
        }
    }
}

impl FanoutReadStore for FileFanoutStore {
    fn load_signature_pages(&self, spec: &FanoutSpec) -> Result<Vec<SignaturePage>, StorageError> {
        let directory = self.target_root(spec).join("signature-pages");
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(StorageError::Io(error.to_string())),
        };
        let mut paths = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        paths.sort();
        paths
            .into_iter()
            .map(|path| {
                let bytes = fs::read(path).map_err(|error| StorageError::Io(error.to_string()))?;
                serde_json::from_slice(&bytes)
                    .map_err(|error| StorageError::Serialization(error.to_string()))
            })
            .collect()
    }

    fn load_fanout_checkpoint(
        &self,
        spec: &FanoutSpec,
    ) -> Result<Option<crate::solana_archive::FanoutCheckpoint>, StorageError> {
        self.load_checkpoint(spec)
    }
}

fn validate_page(spec: &FanoutSpec, page: &SignaturePage) -> Result<(), StorageError> {
    if matches!(
        page.provider,
        ProviderKind::LegacyCompatibility | ProviderKind::Unknown
    ) || page.role != SourceRole::RawIndependent
    {
        return Err(StorageError::InvalidInput(
            "fanout page is not a Helius raw-independent observation".to_string(),
        ));
    }
    if page.query.cluster != spec.cluster || page.query.target != spec.target {
        return Err(StorageError::InvalidInput(
            "fanout page does not match the declared target".to_string(),
        ));
    }
    if page.response_digest.trim().is_empty() {
        return Err(StorageError::InvalidInput(
            "provider response digest is missing".to_string(),
        ));
    }
    Ok(())
}

fn validate_transaction(
    spec: &FanoutSpec,
    transaction: &RawTransactionObservation,
) -> Result<(), StorageError> {
    if matches!(
        transaction.provider,
        ProviderKind::LegacyCompatibility | ProviderKind::Unknown
    ) || transaction.role != SourceRole::RawIndependent
    {
        return Err(StorageError::InvalidInput(
            "fanout transaction is not a Helius raw-independent observation".to_string(),
        ));
    }
    if transaction.query.cluster != spec.cluster || transaction.query.signature.trim().is_empty() {
        return Err(StorageError::InvalidInput(
            "fanout transaction does not match the declared target".to_string(),
        ));
    }
    if transaction.response_digest.trim().is_empty() {
        return Err(StorageError::InvalidInput(
            "provider response digest is missing".to_string(),
        ));
    }
    Ok(())
}

fn target_namespace(spec: &FanoutSpec) -> String {
    let identity = format!("{}\0{}", spec.cluster, spec.target);
    format!("target-{}", blake3::hash(identity.as_bytes()).to_hex())
}

fn content_digest(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

fn write_append_only(path: &Path, bytes: &[u8]) -> Result<bool, StorageError> {
    if path.exists() {
        let existing = fs::read(path).map_err(|error| StorageError::Io(error.to_string()))?;
        if existing == bytes {
            return Ok(true);
        }
        return Err(StorageError::IntegrityConflict(
            "append-only path already contains different bytes".to_string(),
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
    sync_directory(parent)
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
    sync_directory(parent)
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
