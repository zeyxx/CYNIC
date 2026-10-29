//! Embedded SQLite projection engine and transactional state storage for CYNIC.
//!
//! Provides zero-network, local-first SQLite persistence with transactional guarantees.

pub mod book;
pub mod error;
pub mod projection;

pub use book::{BookEntry, CynicLedger, GenericBookProjector, LedgerBook, TypedBook};
pub use error::SqliteError;
pub use projection::{SqliteProjectionAdapter, SqliteStateTransaction};
pub use projection::tabular::{TabularMemoryProjection, TabularQueryResult};
pub use rusqlite;
