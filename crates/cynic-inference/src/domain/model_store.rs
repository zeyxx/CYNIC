//! Centralized model store view: vault weights + profiles + HF provenance.
//!
//! This is the organism-facing inventory of **where models live**, distinct
//! from routing backends (`inference.toml`) and from ODS product state.

use serde::{Deserialize, Serialize};

use super::local_runtime::LocalModelProfile;
<<<<<<< HEAD
use super::model_layout_report::ModelLayoutReport;
=======
use super::model_layout::ModelLayoutReport;
>>>>>>> github/main

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLocationKind {
    /// Under CYNIC vault (`ModelLayout.vault_dir`).
    Vault,
    /// Hugging Face hub cache.
    HfCache,
    /// llama.cpp cache.
    LlamaCache,
    /// Declared path outside known roots.
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelArtifact {
    /// Stable id (filename stem or profile alias).
    pub id: String,
    /// Absolute path on disk.
    pub path: String,
    pub location: ModelLocationKind,
    pub kind: String,
    pub size_bytes: Option<u64>,
    pub exists: bool,
    /// Linked runtime profile alias, if any.
    pub profile_alias: Option<String>,
    /// HF repo id when downloaded via Hub.
    pub hf_repo: Option<String>,
    pub hf_revision: Option<String>,
}

/// Full centralized store report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStoreReport {
    pub schema: &'static str,
    pub surface: &'static str,
    pub layout: ModelLayoutReport,
    pub artifacts: Vec<ModelArtifact>,
    pub profiles: Vec<LocalModelProfile>,
    pub totals: ModelStoreTotals,
}

impl ModelStoreReport {
    pub const SCHEMA: &'static str = "cynic.inference.model_store.v1";
    pub const SURFACE: &'static str = "native";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStoreTotals {
    pub artifacts: usize,
    pub profiles: usize,
    pub vault_bytes: u64,
    pub missing_files: usize,
}
