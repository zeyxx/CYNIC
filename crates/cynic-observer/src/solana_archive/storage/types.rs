use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredPageReceipt {
    pub relative_path: String,
    pub content_digest: String,
    pub already_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    InvalidInput(String),
    Io(String),
    Serialization(String),
    IntegrityConflict(String),
}
