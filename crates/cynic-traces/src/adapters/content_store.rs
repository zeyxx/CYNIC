//! Content-addressed blob store for large payloads.
//!
//! Strict port of `.../adapters/content_store.py`. SQLite stores TEXT
//! without a practical limit, but parameters above ~2 GiB are refused by
//! the binding. Rather than truncate (a semantic loss), large payloads go
//! to content-addressed files and the database keeps only the hash.

use std::path::{Path, PathBuf};

use crate::kernel::ids::content_hash;

/// Payloads up to this size stay in the database. 1 GiB leaves a wide
/// margin under the 2 GiB binding limit.
pub const INLINE_LIMIT: usize = 1 << 30;

#[derive(Debug)]
pub struct ContentBlobStore {
    root: PathBuf,
}

impl ContentBlobStore {
    pub fn new(root: &str) -> std::io::Result<Self> {
        std::fs::create_dir_all(Path::new(root).join("blobs"))?;
        Ok(Self {
            root: PathBuf::from(root),
        })
    }

    fn blob_path(&self, digest: &str) -> PathBuf {
        let hexpart = digest.split_once(':').map(|(_, h)| h).unwrap_or(digest);
        self.root
            .join("blobs")
            .join(&hexpart[..2.min(hexpart.len())])
            .join(hexpart.get(2..).unwrap_or(""))
    }

    /// Store a payload. Returns `(content_hash, inlined)`: `inlined` is
    /// true when the payload stays in the database.
    pub fn put(&self, data: &str) -> std::io::Result<(String, bool)> {
        let raw = data.as_bytes();
        let digest = content_hash(raw);
        if raw.len() <= INLINE_LIMIT {
            return Ok((digest, true));
        }
        let path = self.blob_path(&digest);
        if !path.exists() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, raw)?;
        }
        Ok((digest, false))
    }

    /// Retrieve a payload by hash. Returns None if absent.
    pub fn get(&self, digest: &str) -> Option<String> {
        let path = self.blob_path(digest);
        if !path.exists() {
            return None;
        }
        std::fs::read(&path)
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    }
}
