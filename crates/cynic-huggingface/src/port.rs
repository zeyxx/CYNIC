//! Port for Hugging Face Hub CLI (`hf`) — canonical download/auth surface.
//!
//! Adapter implements argv-based calls to the official `hf` binary
//! (https://huggingface.co/docs/huggingface_hub/guides/cli). Secrets via
//! `HF_TOKEN` env, never logged.

use serde::{Deserialize, Serialize};

use crate::domain::endpoint::HfEndpointSpec;
use crate::domain::job::{HfJobInfo, HfJobSpec, HfScheduledJobRequest};
use crate::domain::paths::HfLocalPaths;
use crate::domain::repository::HfRepoType;
use crate::domain::space::HfSpaceSpec;
use crate::domain::webhook::HfWebhookSpec;

#[derive(Debug, thiserror::Error)]
pub enum HfHubError {
    #[error("hf CLI unavailable: {0}")]
    Unavailable(String),
    #[error("hf command failed: {0}")]
    Command(String),
    #[error("hf rejected: {0}")]
    Rejected(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfCliStatus {
    pub schema: &'static str,
    pub surface: &'static str,
    pub hf_bin: String,
    pub available: bool,
    pub version: Option<String>,
    pub whoami: Option<String>,
    pub authenticated: bool,
    pub vault_dir: String,
    pub hf_cache_dir: String,
    pub message: Option<String>,
}

impl HfCliStatus {
    pub const SCHEMA: &'static str = "cynic.inference.hf_cli_status.v1";
    pub const SURFACE: &'static str = "hf_cli";
}

#[derive(Debug, Clone)]
pub struct HfDownloadRequest {
    pub repo_id: String,
    pub revision: Option<String>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    /// If set, download into vault/hf/<slug>/ (default). If false, use HF cache only.
    pub into_vault: bool,
    pub repo_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfDownloadReport {
    pub schema: &'static str,
    pub surface: &'static str,
    pub ok: bool,
    pub repo_id: String,
    pub local_dir: Option<String>,
    pub argv: Vec<String>,
    pub stdout_tail: String,
    pub stderr_tail: String,
    pub message: Option<String>,
    /// Primary GGUF path when detected after download.
    pub primary_gguf: Option<String>,
    /// Profile alias written (if any).
    pub profile_alias: Option<String>,
    pub profile_path: Option<String>,
}

impl HfDownloadReport {
    pub const SCHEMA: &'static str = "cynic.inference.hf_download.v1";
    pub const SURFACE: &'static str = "hf_cli";
}

#[derive(Debug, Clone)]
pub struct HfSearchRequest {
    pub query: Option<String>,
    pub category: Option<String>,
    pub author: Option<String>,
    pub limit: u32,
    pub sort: Option<String>,
    /// Prefer GGUF-tagged models (default true for local llama.cpp path).
    pub gguf_only: bool,
    /// Hub filter e.g. `min:0B,max:8B` (`hf models list --num-parameters`).
    pub num_parameters: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfModelHit {
    pub repo_id: String,
    /// Short typable alias (not unique globally — use with repo_id).
    pub alias: String,
    pub downloads: Option<u64>,
    pub likes: Option<u64>,
    pub pipeline_tag: Option<String>,
    pub library_name: Option<String>,
    pub tags: Vec<String>,
    pub trending_score: Option<f64>,
    pub estimated_size_bytes: Option<u64>,
    pub hardware_fit: String,
    pub fit_score: u32,
    /// Suggested `--include` globs for download.
    pub suggested_include: Vec<String>,
    pub category_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfSearchReport {
    pub schema: &'static str,
    pub surface: &'static str,
    pub query: String,
    pub category: Option<String>,
    pub hardware_tier: String,
    pub recommended_max_weight_bytes: u64,
    pub hits: Vec<HfModelHit>,
    pub message: Option<String>,
}

impl HfSearchReport {
    pub const SCHEMA: &'static str = "cynic.inference.hf_search.v1";
    pub const SURFACE: &'static str = "hf_cli";
}

/// Hugging Face Hub operations via the official CLI.
pub trait HfHubPort: Send + Sync {
    fn status(&self, paths: &HfLocalPaths) -> Result<HfCliStatus, HfHubError>;
    fn download(
        &self,
        paths: &HfLocalPaths,
        req: &HfDownloadRequest,
    ) -> Result<HfDownloadReport, HfHubError>;
    fn search(&self, req: &HfSearchRequest) -> Result<Vec<serde_json::Value>, HfHubError>;
    fn list_repo_files(
        &self,
        repo_id: &str,
        revision: Option<&str>,
    ) -> Result<Vec<String>, HfHubError>;
    /// `hf upload ...` — publish a file or folder to the Hub.
    fn upload(&self, req: &HfUploadRequest) -> Result<HfUploadReport, HfHubError>;
    /// `hf jobs uv run ...` / `hf jobs run ...` — submit, return remote id.
    fn job_submit(&self, spec: &HfJobSpec) -> Result<HfJobReport, HfHubError>;
    /// `hf jobs inspect ID --json` — observed state, raw payload preserved.
    fn job_poll(&self, remote_id: &str, namespace: Option<&str>) -> Result<HfJobInfo, HfHubError>;
    /// `hf jobs ps -a --json` — list observed jobs.
    fn job_list(&self, all: bool, namespace: Option<&str>) -> Result<Vec<HfJobInfo>, HfHubError>;
    /// `hf jobs logs ID` — captured log tail.
    fn job_logs(
        &self,
        remote_id: &str,
        namespace: Option<&str>,
    ) -> Result<HfJobLogsReport, HfHubError>;
    /// `hf jobs cancel ID`.
    fn job_cancel(
        &self,
        remote_id: &str,
        namespace: Option<&str>,
    ) -> Result<HfJobReport, HfHubError>;
    /// `hf jobs scheduled uv run SCHEDULE ...`.
    fn job_schedule_submit(&self, req: &HfScheduledJobRequest) -> Result<HfJobReport, HfHubError>;
    /// `hf jobs scheduled ps --json`.
    fn job_schedule_list(&self) -> Result<Vec<serde_json::Value>, HfHubError>;
    /// `hf jobs scheduled delete ID`.
    fn job_schedule_delete(&self, scheduled_id: &str) -> Result<HfJobReport, HfHubError>;
}

/// Staged contract for Spaces (`hf spaces ...`). No CLI adapter yet.
pub trait HfSpacePort: Send + Sync {
    fn space_create(&self, spec: &HfSpaceSpec) -> Result<HfSpaceReport, HfHubError>;
    fn space_pause(&self, repo_id: &str) -> Result<HfSpaceReport, HfHubError>;
    fn space_restart(&self, repo_id: &str) -> Result<HfSpaceReport, HfHubError>;
}

/// Staged contract for webhooks (`hf webhooks ...`). No CLI adapter yet.
pub trait HfWebhookPort: Send + Sync {
    fn webhook_create(&self, spec: &HfWebhookSpec) -> Result<HfWebhookReport, HfHubError>;
    fn webhook_list(&self) -> Result<Vec<serde_json::Value>, HfHubError>;
    fn webhook_delete(&self, webhook_id: &str) -> Result<HfWebhookReport, HfHubError>;
}

/// Staged contract for Inference Endpoints (`hf endpoints ...`). No CLI adapter yet.
pub trait HfEndpointPort: Send + Sync {
    fn endpoint_create(&self, spec: &HfEndpointSpec) -> Result<HfEndpointReport, HfHubError>;
    fn endpoint_list(&self) -> Result<Vec<serde_json::Value>, HfHubError>;
    fn endpoint_delete(&self, name: &str) -> Result<HfEndpointReport, HfHubError>;
}

#[derive(Debug, Clone)]
pub struct HfUploadRequest {
    pub repo_id: String,
    pub local_path: String,
    pub path_in_repo: Option<String>,
    pub repo_type: HfRepoType,
    pub revision: Option<String>,
    /// `Some(true)` creates a private repo when missing; `None` = Hub default.
    pub private: Option<bool>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub commit_message: Option<String>,
    pub create_pr: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfUploadReport {
    pub schema: &'static str,
    pub surface: &'static str,
    pub ok: bool,
    pub repo_id: String,
    pub argv: Vec<String>,
    pub stdout_tail: String,
    pub stderr_tail: String,
    pub message: Option<String>,
}

impl HfUploadReport {
    pub const SCHEMA: &'static str = "cynic.huggingface.hf_upload.v1";
    pub const SURFACE: &'static str = "hf_cli";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfJobReport {
    pub schema: &'static str,
    pub surface: &'static str,
    pub ok: bool,
    pub remote_id: Option<String>,
    pub argv: Vec<String>,
    pub stdout_tail: String,
    pub stderr_tail: String,
    pub message: Option<String>,
}

impl HfJobReport {
    pub const SCHEMA: &'static str = "cynic.huggingface.hf_job.v1";
    pub const SURFACE: &'static str = "hf_cli";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfJobLogsReport {
    pub schema: &'static str,
    pub surface: &'static str,
    pub ok: bool,
    pub remote_id: String,
    pub logs: String,
    pub message: Option<String>,
}

impl HfJobLogsReport {
    pub const SCHEMA: &'static str = "cynic.huggingface.hf_job_logs.v1";
    pub const SURFACE: &'static str = "hf_cli";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfSpaceReport {
    pub schema: &'static str,
    pub surface: &'static str,
    pub ok: bool,
    pub repo_id: String,
    pub message: Option<String>,
}

impl HfSpaceReport {
    pub const SCHEMA: &'static str = "cynic.huggingface.hf_space.v1";
    pub const SURFACE: &'static str = "hf_cli";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfWebhookReport {
    pub schema: &'static str,
    pub surface: &'static str,
    pub ok: bool,
    pub remote_id: Option<String>,
    pub message: Option<String>,
}

impl HfWebhookReport {
    pub const SCHEMA: &'static str = "cynic.huggingface.hf_webhook.v1";
    pub const SURFACE: &'static str = "hf_cli";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfEndpointReport {
    pub schema: &'static str,
    pub surface: &'static str,
    pub ok: bool,
    pub name: String,
    pub message: Option<String>,
}

impl HfEndpointReport {
    pub const SCHEMA: &'static str = "cynic.huggingface.hf_endpoint.v1";
    pub const SURFACE: &'static str = "hf_cli";
}
