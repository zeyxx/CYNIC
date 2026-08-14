//! Boot-time backend health logging.
//!
//! Runtime HTTP verification belongs to `cynic-node::verify`;
//! this kernel adapter keeps the non-blocking boot log projection only.

use crate::infra::config::BackendConfig;
use cynic_node::verify::{IdentityResult, check_health, check_identity_with_bearer};

/// Validate config at boot — probe health URLs, log warnings for unreachable backends.
/// Does NOT block boot — sovereign degradation is preferred over refusing to start.
pub async fn validate_config(configs: Vec<BackendConfig>) {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "HTTP client build failed (TLS?) — skipping health validation");
            return;
        }
    };

    for cfg in &configs {
        let Some(health_url) = cfg.health_url.as_deref() else {
            klog!(
                "[config] — {} no health probe (cloud API, error-driven circuit breaker)",
                cfg.name
            );
            continue;
        };
        if check_health(&client, health_url, 5).await {
            klog!("[config] ✓ {} health OK ({})", cfg.name, health_url);
            verify_model_loaded(&client, cfg).await;
        } else {
            klog!(
                "[config] ✗ {} UNREACHABLE at {} — will load anyway, health loop will recover",
                cfg.name,
                health_url
            );
        }
    }
}

async fn verify_model_loaded(client: &reqwest::Client, cfg: &BackendConfig) {
    let models_url = format!("{}/models", cfg.base_url.trim_end_matches('/'));
    match check_identity_with_bearer(client, &models_url, &cfg.model, 5, cfg.api_key.as_deref())
        .await
    {
        IdentityResult::Match => {
            klog!(
                "[config] ✓ {} model '{}' verified loaded",
                cfg.name,
                cfg.model
            );
        }
        IdentityResult::Mismatch { actual, .. } => {
            klog!(
                "[config] ⚠ {} model '{}' NOT FOUND in /models response — config drift?",
                cfg.name,
                cfg.model
            );
            tracing::warn!(
                backend = %cfg.name,
                configured_model = %cfg.model,
                actual_model = %actual,
                "Model not found in backend /models — config may not match running server"
            );
        }
        IdentityResult::Unknown | IdentityResult::Unreachable => {}
    }
}
