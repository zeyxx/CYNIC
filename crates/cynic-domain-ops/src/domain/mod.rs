//! Pure domain logic for external domain operations.
//!
//! This module is 100% pure: no I/O, no `std::process`, no `reqwest`, no
//! `tokio::spawn`. All decision logic (record transformation, zone
//! reconciliation, idempotency) lives here so it can be unit-tested
//! deterministically without network access.

pub mod idempotency;
pub mod reconcile;
pub mod transform;

pub use idempotency::generate_idempotency_key;
pub use reconcile::{ConflictDetection, reconcile_zone};
pub use transform::{RecordCategory, TransformResult, classify_record, policy_for_record_type, transform_record_for_cloudflare};
