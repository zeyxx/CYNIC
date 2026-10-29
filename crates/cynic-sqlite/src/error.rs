//! SQLite projection and transaction errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SqliteError {
    #[error("SQLite database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Storage I/O failure: {0}")]
    Io(String),

    #[error("Transaction failed: {0}")]
    Transaction(String),

    #[error("Table or projection error: {0}")]
    Projection(String),
}
