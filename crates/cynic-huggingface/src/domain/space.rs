//! Pure contract for Hugging Face Spaces (`hf spaces ...`).
//!
//! Staged: port defined, no CLI adapter yet. Secrets by NAME only.

use serde::{Deserialize, Serialize};

/// Creation / management spec for a Space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HfSpaceSpec {
    pub repo_id: String,
    /// `gradio`, `streamlit`, `docker`, ... `None` = Hub default.
    pub sdk: Option<String>,
    /// Hardware slug, e.g. `cpu-basic`, `t4-small`. `None` = Hub default.
    pub hardware: Option<String>,
    /// Secret NAMES resolved from the process environment.
    pub secret_names: Vec<String>,
    /// Plain variables (`--variable K=V`).
    pub variables: Vec<(String, String)>,
    pub private: bool,
}

impl HfSpaceSpec {
    pub fn validated(
        repo_id: impl Into<String>,
        sdk: Option<String>,
    ) -> Result<Self, HfSpaceSpecError> {
        let repo_id = repo_id.into();
        if repo_id.trim().is_empty() {
            return Err(HfSpaceSpecError::EmptyRepoId);
        }
        Ok(Self {
            repo_id,
            sdk,
            hardware: None,
            secret_names: Vec::new(),
            variables: Vec::new(),
            private: false,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HfSpaceStatus {
    Running,
    Paused,
    Building,
    Error,
    Unknown,
}

pub fn map_space_stage(stage: &str) -> HfSpaceStatus {
    match stage.trim().to_ascii_lowercase().as_str() {
        "running" | "sleeping" => HfSpaceStatus::Running,
        "paused" | "stopped" => HfSpaceStatus::Paused,
        "building" | "starting" | "pending" => HfSpaceStatus::Building,
        "error" | "failed" => HfSpaceStatus::Error,
        _ => HfSpaceStatus::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HfSpaceSpecError {
    #[error("space repo_id required")]
    EmptyRepoId,
}
