//! Pure contract for Hugging Face Inference Endpoints (`hf endpoints ...`).
//!
//! Staged: port defined, no CLI adapter yet.

use serde::{Deserialize, Serialize};

/// Deduplicated endpoint creation spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HfEndpointSpec {
    pub name: String,
    pub repository: String,
    pub framework: Option<String>,
    pub task: Option<String>,
    pub accelerator: Option<String>,
    pub vendor: Option<String>,
    pub region: Option<String>,
    pub instance_type: Option<String>,
    pub instance_size: Option<String>,
}

impl HfEndpointSpec {
    pub fn validated(
        name: impl Into<String>,
        repository: impl Into<String>,
    ) -> Result<Self, HfEndpointSpecError> {
        let name = name.into();
        let repository = repository.into();
        if name.trim().is_empty() {
            return Err(HfEndpointSpecError::EmptyName);
        }
        if repository.trim().is_empty() {
            return Err(HfEndpointSpecError::EmptyRepository);
        }
        Ok(Self {
            name,
            repository,
            framework: None,
            task: None,
            accelerator: None,
            vendor: None,
            region: None,
            instance_type: None,
            instance_size: None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HfEndpointStatus {
    Running,
    ScaledToZero,
    Building,
    Failed,
    Unknown,
}

pub fn map_endpoint_state(state: &str) -> HfEndpointStatus {
    match state.trim().to_ascii_lowercase().as_str() {
        "running" | "healthy" => HfEndpointStatus::Running,
        "scaled-to-zero" | "paused" | "sleeping" => HfEndpointStatus::ScaledToZero,
        "pending" | "building" | "initializing" | "updating" => HfEndpointStatus::Building,
        "failed" | "error" => HfEndpointStatus::Failed,
        _ => HfEndpointStatus::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HfEndpointSpecError {
    #[error("endpoint name required")]
    EmptyName,
    #[error("endpoint repository required")]
    EmptyRepository,
}
