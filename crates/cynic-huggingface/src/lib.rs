//! Hugging Face external provider boundary for CYNIC.
//!
//! This crate owns the data-centric contract for Hugging Face artifacts:
//! - repository and revision identity
//! - dataset specifications
//! - async job requests and handles
//! - export reports required before any external publication
//! - the canonical CLI adapter for download/search/status operations
//!
//! The domain modules are pure. The adapter is the only module allowed to
//! invoke the external `hf` CLI.

pub mod adapters;
pub mod domain;
pub mod port;

pub use adapters::cli::HfCliAdapter;
pub use domain::dataset::HfDatasetSpec;
pub use domain::export::ExternalExportReport;
pub use domain::job::{HfJobHandle, HfJobSpec, HfJobStatus};
pub use domain::paths::HfLocalPaths;
pub use domain::projection::ProjectionReadiness;
pub use domain::repository::HfRepositoryRef;
pub use port::{
    HfCliStatus, HfDownloadReport, HfDownloadRequest, HfHubError, HfHubPort, HfModelHit,
    HfSearchReport, HfSearchRequest,
};