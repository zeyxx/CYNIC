//! Pure contract for Hugging Face webhooks (`hf webhooks ...`).
//!
//! Staged: port defined, no CLI adapter yet. A webhook delivers to a URL
//! or triggers a Job (`job_id`); exactly one target is expected.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HfWebhookSpec {
    pub url: Option<String>,
    pub job_id: Option<String>,
    /// e.g. `user:acme`, `org:cynic`, `repo:acme/ds`.
    pub watched: Vec<String>,
    /// e.g. `repo`, `discussion`.
    pub domains: Vec<String>,
    pub enabled: bool,
}

impl HfWebhookSpec {
    pub fn validated(
        url: Option<String>,
        job_id: Option<String>,
    ) -> Result<Self, HfWebhookSpecError> {
        match (&url, &job_id) {
            (None, None) => Err(HfWebhookSpecError::NoTarget),
            (Some(_), Some(_)) => Err(HfWebhookSpecError::BothTargets),
            _ => Ok(Self {
                url,
                job_id,
                watched: Vec::new(),
                domains: vec!["repo".into()],
                enabled: true,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HfWebhookInfo {
    pub remote_id: String,
    pub enabled: bool,
    #[serde(default)]
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HfWebhookSpecError {
    #[error("webhook needs a url or a job_id target")]
    NoTarget,
    #[error("webhook takes either a url or a job_id, not both")]
    BothTargets,
}
