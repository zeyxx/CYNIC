//! Hugging Face external provider boundary for CYNIC.
//!
//! This crate owns the data-centric contract for Hugging Face artifacts:
//! - repository and revision identity (models, datasets, spaces)
//! - dataset specifications and upload (publication) requests
//! - Jobs: one-shot (`uv run` / `run`) and scheduled submissions, polling,
//!   logs, cancellation
//! - export reports required before any external publication
//! - staged contracts (no CLI adapter yet): Spaces, webhooks, Inference Endpoints
//! - the canonical CLI adapter for download/upload/search/jobs operations
//!
//! The domain modules are pure. The adapter is the only module allowed to
//! invoke the external `hf` CLI. Secrets travel by NAME only and resolve
//! from the process environment inside the CLI call.

pub mod adapters;
pub mod domain;
pub mod port;

pub use adapters::cli::HfCliAdapter;
pub use domain::dataset::HfDatasetSpec;
pub use domain::endpoint::{HfEndpointSpec, HfEndpointStatus, map_endpoint_state};
pub use domain::export::ExternalExportReport;
pub use domain::job::{
    HfJobHandle, HfJobInfo, HfJobKind, HfJobSpec, HfJobSpecError, HfJobStatus,
    HfScheduledJobRequest, map_stage_str,
};
pub use domain::paths::HfLocalPaths;
pub use domain::projection::ProjectionReadiness;
pub use domain::repository::{HfRepoType, HfRepositoryRef};
pub use domain::space::{HfSpaceSpec, HfSpaceStatus, map_space_stage};
pub use domain::webhook::{HfWebhookInfo, HfWebhookSpec};
pub use port::{
    HfCliStatus, HfDownloadReport, HfDownloadRequest, HfEndpointPort, HfEndpointReport, HfHubError,
    HfHubPort, HfJobLogsReport, HfJobReport, HfModelHit, HfSearchReport, HfSearchRequest,
    HfSpacePort, HfSpaceReport, HfUploadReport, HfUploadRequest, HfWebhookPort, HfWebhookReport,
};
