//! Runtime verification helpers for node-managed inference backends.
//! This module is the node-side façade for `cynic-inference::application::backend_verification`.

use reqwest::Client;

use cynic_inference::application::backend_verification as inner;

pub const DEFAULT_CONTEXT_POINTER: &str = "/default_generation_settings/n_ctx";

pub async fn check_health(client: &Client, health_url: &str, timeout_secs: u64) -> bool {
    inner::check_health(client, health_url, timeout_secs).await
}

/// Authenticated health observation for a node-managed backend.
pub async fn check_health_with_bearer(
    client: &Client,
    health_url: &str,
    timeout_secs: u64,
    bearer_token: Option<&str>,
) -> bool {
    inner::check_health_with_bearer(client, health_url, timeout_secs, bearer_token).await
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityResult {
    Match,
    Mismatch { expected: String, actual: String },
    Unknown,
    Unreachable,
}

impl From<inner::IdentityResult> for IdentityResult {
    fn from(result: inner::IdentityResult) -> Self {
        match result {
            inner::IdentityResult::Match => Self::Match,
            inner::IdentityResult::Mismatch { expected, actual } => {
                Self::Mismatch { expected, actual }
            }
            inner::IdentityResult::Unknown => Self::Unknown,
            inner::IdentityResult::Unreachable => Self::Unreachable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextResult {
    Observed { n_ctx: u32 },
    Unknown,
    Unreachable,
}

impl From<inner::ContextResult> for ContextResult {
    fn from(result: inner::ContextResult) -> Self {
        match result {
            inner::ContextResult::Observed { n_ctx } => Self::Observed { n_ctx },
            inner::ContextResult::Unknown => Self::Unknown,
            inner::ContextResult::Unreachable => Self::Unreachable,
        }
    }
}

pub async fn check_identity(
    client: &Client,
    models_url: &str,
    expected_model: &str,
    timeout_secs: u64,
) -> IdentityResult {
    inner::check_identity(client, models_url, expected_model, timeout_secs)
        .await
        .into()
}

pub async fn check_identity_with_bearer(
    client: &Client,
    models_url: &str,
    expected_model: &str,
    timeout_secs: u64,
    bearer_token: Option<&str>,
) -> IdentityResult {
    inner::check_identity_with_bearer(
        client,
        models_url,
        expected_model,
        timeout_secs,
        bearer_token,
    )
    .await
    .into()
}

pub async fn check_context(client: &Client, props_url: &str, timeout_secs: u64) -> ContextResult {
    inner::check_context(client, props_url, timeout_secs)
        .await
        .into()
}

pub async fn check_context_with_bearer(
    client: &Client,
    props_url: &str,
    timeout_secs: u64,
    bearer_token: Option<&str>,
) -> ContextResult {
    inner::check_context_with_bearer(client, props_url, timeout_secs, bearer_token)
        .await
        .into()
}

pub fn model_identity_matches(actual_model: &str, expected_model: &str) -> bool {
    inner::model_identity_matches(actual_model, expected_model)
}
