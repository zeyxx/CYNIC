//! Hugging Face external provider boundary for CYNIC.
//!
//! This crate owns the data-centric contract for Hugging Face artifacts:
//! - repository and revision identity
//! - dataset specifications
//! - async job requests and handles
//! - export reports required before any external publication
//!
//! It intentionally contains no I/O. The CLI/SDK adapter and all process
//! invocation stay behind a port implemented elsewhere.

pub mod domain;

pub use domain::dataset::HfDatasetSpec;
pub use domain::export::ExternalExportReport;
pub use domain::repository::HfRepositoryRef;
pub use domain::job::{HfJobHandle, HfJobSpec};
pub use domain::projection::ProjectionReadiness;