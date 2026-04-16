//! Backend health + model verification — boot-time HTTP probes.
//!
//! Lives in the backends adapter layer (K2): the only place `reqwest::Client`
//! is allowed to be constructed. `infra/config` declares the shape of backend
//! configuration; the HTTP probing of that config belongs here.

use crate::infra::config::BackendConfig;

/// Validate config at boot — probe health URLs, log warnings for unreachable backends.
/// Does NOT block boot — sovereign degradation is preferred over refusing to start.
pub async fn validate_config(configs: &[BackendConfig]) {
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

    for cfg in configs {
        let Some(ref health_url) = cfg.health_url else {
            klog!(
                "[config] — {} no health probe (cloud API, error-driven circuit breaker)",
                cfg.name
            );
            continue;
        };
        match client.get(health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                klog!("[config] ✓ {} health OK ({})", cfg.name, health_url);
                // RC3: Verify configured model is actually loaded (sovereign backends only)
                verify_model_loaded(&client, cfg).await;
            }
            Ok(resp) => {
                klog!(
                    "[config] ⚠ {} health returned {} ({})",
                    cfg.name,
                    resp.status(),
                    health_url
                );
            }
            Err(_) => {
                klog!(
                    "[config] ✗ {} UNREACHABLE at {} — will load anyway, health loop will recover",
                    cfg.name,
                    health_url
                );
            }
        }
    }
}

/// RC3 gate: verify the configured model name is actually loaded on the backend.
/// Checks /v1/models (OpenAI-compatible) for backends with health URLs.
/// Non-fatal: logs warning if model is missing or endpoint unavailable.
async fn verify_model_loaded(client: &reqwest::Client, cfg: &BackendConfig) {
    let models_url = format!("{}/models", cfg.base_url.trim_end_matches('/'));
    let Ok(resp) = client.get(&models_url).send().await else {
        return; // Already logged as unreachable in health check
    };
    if !resp.status().is_success() {
        return; // /v1/models not supported — skip silently
    }
    let Ok(body) = resp.text().await else {
        return;
    };
    // Check if the configured model name appears in the response.
    // OpenAI-compatible: {"data": [{"id": "model-name", ...}]}
    // llama-server: {"data": [{"id": "model-name"}]} or flat list
    if body.contains(&cfg.model) {
        klog!(
            "[config] ✓ {} model '{}' verified loaded",
            cfg.name,
            cfg.model
        );
    } else {
        klog!(
            "[config] ⚠ {} model '{}' NOT FOUND in /models response — config drift?",
            cfg.name,
            cfg.model
        );
        tracing::warn!(
            backend = %cfg.name,
            configured_model = %cfg.model,
            "Model not found in backend /models — config may not match running server"
        );
    }
}
