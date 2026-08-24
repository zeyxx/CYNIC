//! Durable, one-shot persistence for BootstrapSeed receipts.
//!
//! A receipt file is created exactly once per request: `create_new` refuses to
//! overwrite an existing receipt, so a replayed seed request can never silently
//! replace the proof of a previous run. Read-back equality is mandatory before
//! the write is considered proved.

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use crate::usecases::bootstrap_seed_uc::BOOTSTRAP_SEED_RECEIPT_SCHEMA;

#[derive(Debug)]
pub struct LocalBootstrapSeedReceiptStore {
    root: PathBuf,
}

impl LocalBootstrapSeedReceiptStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Persists a serialized receipt under `<root>/<request_id>.receipt.json`.
    ///
    /// Fails with `bootstrap_seed_receipt_exists` when a receipt for this
    /// request already exists (one-shot enforcement), and requires byte-exact
    /// read-back before returning success.
    pub fn persist(&self, request_id: &str, receipt_json: &str) -> Result<PathBuf, String> {
        if !request_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            || request_id.is_empty()
        {
            return Err("bootstrap_seed_receipt_request_id_unsafe".to_string());
        }
        let root = self.root.as_path();
        if !root.is_absolute() {
            return Err("bootstrap_seed_receipt_root_not_absolute".to_string());
        }
        fs::create_dir_all(root)
            .map_err(|error| format!("bootstrap_seed_receipt_dir_create_failed:{error}"))?;
        let metadata = fs::symlink_metadata(root)
            .map_err(|error| format!("bootstrap_seed_receipt_root_unobservable:{error}"))?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("bootstrap_seed_receipt_root_unsafe".to_string());
        }
        let path = root.join(format!("{request_id}.receipt.json"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::AlreadyExists => "bootstrap_seed_receipt_exists".to_string(),
                _ => format!("bootstrap_seed_receipt_create_failed:{error}"),
            })?;
        file.write_all(receipt_json.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("bootstrap_seed_receipt_write_failed:{error}"))?;
        File::open(root)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("bootstrap_seed_receipt_dir_sync_failed:{error}"))?;
        let read_back = fs::read_to_string(&path)
            .map_err(|error| format!("bootstrap_seed_receipt_read_back_failed:{error}"))?;
        if read_back != receipt_json {
            return Err("bootstrap_seed_receipt_read_back_mismatch".to_string());
        }
        Ok(path)
    }

    pub fn read(&self, request_id: &str) -> Result<Option<String>, String> {
        if !request_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            || request_id.is_empty()
        {
            return Err("bootstrap_seed_receipt_request_id_unsafe".to_string());
        }
        let path = self.root.join(format!("{request_id}.receipt.json"));
        match fs::read_to_string(&path) {
            Ok(content) => Ok(Some(content)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("bootstrap_seed_receipt_read_failed:{error}")),
        }
    }
}

pub(crate) const RECEIPT_SCHEMA_MARKER: &str = BOOTSTRAP_SEED_RECEIPT_SCHEMA;

fn _schema_marker_reference() -> &'static str {
    RECEIPT_SCHEMA_MARKER
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUEST_ID: &str = "seed-1";

    #[test]
    fn persists_once_and_refuses_overwrite_with_exact_read_back() {
        let root = tempfile::tempdir().expect("temp root");
        let store = LocalBootstrapSeedReceiptStore::new(root.path());

        let path = store
            .persist(REQUEST_ID, "{\"status\":\"proved\"}")
            .expect("first persist");
        assert_eq!(path, root.path().join(format!("{REQUEST_ID}.receipt.json")));
        assert_eq!(
            store.read(REQUEST_ID).expect("read back"),
            Some("{\"status\":\"proved\"}".to_string())
        );

        let second = store.persist(REQUEST_ID, "{\"status\":\"proved\"}");
        assert_eq!(second.unwrap_err(), "bootstrap_seed_receipt_exists");
    }

    #[test]
    fn unsafe_request_ids_and_relative_roots_are_rejected() {
        let relative = LocalBootstrapSeedReceiptStore::new("relative-root");
        assert_eq!(
            relative.persist(REQUEST_ID, "{}").unwrap_err(),
            "bootstrap_seed_receipt_root_not_absolute"
        );

        let absolute = tempfile::tempdir().expect("temp root");
        let store = LocalBootstrapSeedReceiptStore::new(absolute.path());
        assert_eq!(
            store.persist("../escape", "{}").unwrap_err(),
            "bootstrap_seed_receipt_request_id_unsafe"
        );
    }
}
