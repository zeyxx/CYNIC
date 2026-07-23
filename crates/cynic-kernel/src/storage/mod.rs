//! Storage adapters and helpers.
//!
//! Surreal storage is implemented as an HTTP SurrealDB 3.x client (POST /sql).
//! The split below keeps protocol, bootstrap, metrics, and validation concerns in
//! dedicated modules.

mod client;
mod metrics;
mod schema;
mod validation;

pub mod memory;
pub mod reconnectable;
pub mod surreal;

pub use client::SurrealHttpStorage;
pub use metrics::{StorageMetrics, StorageMetricsSnapshot, TableOpMetrics};
pub(crate) use validation::{escape_surreal, safe_limit, sanitize_id};
