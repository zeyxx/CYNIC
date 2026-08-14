//! Port for Hugging Face Hub CLI (`hf`) — canonical download/auth surface.
//!
//! Adapter implements argv-based calls to the official `hf` binary
//! (https://huggingface.co/docs/huggingface_hub/guides/cli). Secrets via
//! `HF_TOKEN` env, never logged.

use serde::{Deserialize, Serialize};

use crate::domain::model_layout::ModelLayout;

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
    fn status(&self, layout: &ModelLayout) -> Result<HfCliStatus, HfHubError>;
    fn download(
        &self,
        layout: &ModelLayout,
        req: &HfDownloadRequest,
    ) -> Result<HfDownloadReport, HfHubError>;
    fn search(&self, req: &HfSearchRequest) -> Result<Vec<serde_json::Value>, HfHubError>;
    fn list_repo_files(
        &self,
        repo_id: &str,
        revision: Option<&str>,
    ) -> Result<Vec<String>, HfHubError>;
}
