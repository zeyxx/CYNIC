//! Boot extraction: Ring 2 assembly from backend configs, integrity chain seed,
//! fleet probe targets. Called once at kernel start; keeps `main.rs` as a linear
//! composition root instead of a factory site.

use std::collections::HashMap;
use std::sync::Arc;

use crate::backends;
use crate::dogs;
use crate::domain::{self, inference::BackendPort};
use crate::infra::config::{BackendConfig, BackendRemediation, BackendType};
use crate::infra::probes::FleetTarget;
use crate::judge;
use crate::organ;

/// Determine if a backend URL points to sovereign infrastructure.
/// Sovereign = local network (127.x, 10.x, 100.x Tailscale, 192.168.x) or localhost.
/// CLI backends (gemini-cli) use file paths, not URLs — treated as non-sovereign.
/// This is alive: adding a new local backend automatically classifies it as sovereign.
pub fn is_sovereign_url(url: &str) -> bool {
    let url_lower = url.to_lowercase();
    // Local/private network ranges
    url_lower.starts_with("http://127.")
        || url_lower.starts_with("http://localhost")
        || url_lower.starts_with("http://10.")
        || url_lower.starts_with("http://100.") // Tailscale CGNAT range
        || url_lower.starts_with("http://192.168.")
        || url_lower.starts_with("http://172.") // Docker/k8s private
}

/// Everything derived from `backend_configs` at boot: the judge's Dogs, organ
/// backend handles, and the ancillary maps downstream systems consume
/// (health probes, fleet drift detection, remediation watchers, usage costs).
pub struct DogsAndOrgan {
    pub dogs: Vec<Arc<dyn domain::dog::Dog>>,
    pub organ_handles: Vec<Option<organ::BackendHandle>>,
    pub organ: Arc<organ::InferenceOrgan>,
    pub cost_rates: Vec<(String, f64, f64)>,
    pub health_urls: HashMap<String, Option<String>>,
    /// `(base_url, context_size, model, api_key)` per backend — feeds fleet probes
    /// and the discovery loop. CLI backends are intentionally absent (no HTTP URL).
    pub fleet_meta: HashMap<String, (String, u32, String, Option<String>)>,
    pub remediation_configs: HashMap<String, BackendRemediation>,
}

impl std::fmt::Debug for DogsAndOrgan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DogsAndOrgan").finish_non_exhaustive()
    }
}

/// Build the Dog ensemble + organ registry from `backend_configs`.
///
/// - Always starts with `DeterministicDog` (free, fast, no backend).
/// - For each config, constructs the matching `ChatPort` adapter
///   (CLI or OpenAI-compat), probes health once (non-fatal — health loop will
///   recover later), and wires the Dog + organ backend.
/// - Restores persisted `DogStats` from storage so quality knowledge survives
///   restarts (B5 amnesia fix).
pub async fn build_dogs_and_organ(
    backend_configs: Vec<BackendConfig>,
    domain_prompts: &Arc<HashMap<String, String>>,
    storage_port: &Arc<dyn domain::storage::StoragePort>,
) -> DogsAndOrgan {
    let mut dogs: Vec<Arc<dyn domain::dog::Dog>> = Vec::new();
    let mut organ_handles: Vec<Option<organ::BackendHandle>> = Vec::new();
    let mut organ = organ::InferenceOrgan::boot_empty();

    // Always add the deterministic Dog (free, fast)
    dogs.push(Arc::new(dogs::deterministic::DeterministicDog));
    organ_handles.push(None); // deterministic dog has no backend to track
    klog!("[Ring 2] DeterministicDog loaded");

    // Create InferenceDog per configured backend
    // Also collect health URLs and remediation configs from the SoT (backends.toml)
    let mut cost_rates: Vec<(String, f64, f64)> = Vec::new();
    let mut remediation_configs: HashMap<String, BackendRemediation> = HashMap::new();
    let mut health_urls: HashMap<String, Option<String>> = HashMap::new();
    // Fleet probe needs base_url (for /props + /v1/models), context_size, model name, api_key
    let mut fleet_meta: HashMap<String, (String, u32, String, Option<String>)> = HashMap::new();
    for cfg in backend_configs {
        let backend: Arc<dyn domain::chat::ChatPort> = match cfg.backend_type {
            BackendType::Cli => Arc::new(backends::cli::CliBackend::new(
                &cfg.name,
                &cfg.base_url, // for CLI backends, base_url holds the binary path/name
                cfg.timeout_secs,
            )),
            BackendType::OpenAi => match backends::openai::OpenAiCompatBackend::new(cfg.clone()) {
                Ok(b) => Arc::new(b),
                Err(e) => {
                    klog!(
                        "[Ring 2] InferenceDog '{}' SKIPPED — HTTP client init failed: {}",
                        cfg.name,
                        e
                    );
                    continue;
                }
            },
        };
        let health = BackendPort::health(backend.as_ref()).await;
        // Always load the Dog — health loop will recover it if unreachable.
        // Skipping at boot prevents the health loop from ever probing it.
        match health {
            domain::inference::BackendStatus::Healthy
            | domain::inference::BackendStatus::Degraded { .. } => {
                klog!(
                    "[Ring 2] InferenceDog '{}' loaded (model: {}, health: {:?})",
                    cfg.name,
                    cfg.model,
                    health
                );
            }
            _ => {
                klog!(
                    "[Ring 2] InferenceDog '{}' loaded (model: {}, health: {:?}) — health loop will recover",
                    cfg.name,
                    cfg.model,
                    health
                );
            }
        }
        cost_rates.push((
            cfg.name.clone(),
            cfg.cost_input_per_mtok,
            cfg.cost_output_per_mtok,
        ));
        health_urls.insert(cfg.name.clone(), cfg.health_url.clone());
        // CLI backends have no HTTP URL to probe — skip fleet_meta insertion
        if cfg.backend_type != BackendType::Cli {
            fleet_meta.insert(
                cfg.name.clone(),
                (
                    cfg.base_url.clone(),
                    cfg.context_size,
                    cfg.model.clone(),
                    cfg.api_key.clone(),
                ),
            );
        }
        if let Some(rem) = cfg.remediation.clone() {
            remediation_configs.insert(cfg.name.clone(), rem);
        }
        // Sovereign = local/Tailscale URL, no cloud dependency.
        // Heuristic: URLs starting with http://10. http://100. http://192.168. http://127.
        // or http://localhost are sovereign. Everything else (https://*.huggingface.co,
        // gemini CLI, etc.) is non-sovereign. Alive: reads from config, not hardcoded list.
        let sovereign = is_sovereign_url(&cfg.base_url);
        let inference_dog = dogs::inference::InferenceDog::new(
            backend,
            cfg.name.clone(),
            cfg.context_size,
            cfg.timeout_secs,
            cfg.prompt_tier,
            sovereign,
        )
        .with_domain_prompts(Arc::clone(domain_prompts));
        let budget_handle = inference_dog.budget_handle();
        let thinking_handle = inference_dog.thinking_handle();
        dogs.push(Arc::new(inference_dog));

        // Register backend in organ — maps BackendConfig → organ registry entry
        let organ_backend = organ::registry::Backend {
            id: organ::registry::BackendId(cfg.name.clone()),
            declared: organ::registry::DeclaredCapabilities {
                json: cfg.json_mode,
                thinking: !cfg.disable_thinking,
                scoring: true,
                ..Default::default()
            },
            measured: organ::registry::MeasuredCapabilities::default(),
            health: organ::registry::BackendHealth::Healthy,
        };
        let handle = organ.register_backend(organ_backend);
        // Wire dynamic budget + thinking: organ pushes calibrated budget, reads thinking max
        organ::InferenceOrgan::attach_budget_handle(&handle, budget_handle);
        organ::InferenceOrgan::attach_thinking_handle(&handle, thinking_handle);
        organ_handles.push(Some(handle));
    }

    klog!(
        "[Ring 2] {} Dog(s) active, InferenceOrgan: {} backend(s) registered",
        dogs.len(),
        organ.backend_count()
    );

    // Load persisted DogStats from DB — restores quality knowledge across restarts (B5 amnesia fix)
    match storage_port.load_dog_stats().await {
        Ok(loaded) if !loaded.is_empty() => {
            organ.restore_stats(&loaded);
            klog!(
                "[Ring 2] Organ: restored {} Dog quality histories",
                loaded.len()
            );
        }
        Err(e) => klog!(
            "[Ring 2] Organ: failed to load Dog stats (non-fatal): {}",
            e
        ),
        _ => {}
    }
    let organ = Arc::new(organ);

    DogsAndOrgan {
        dogs,
        organ_handles,
        organ,
        cost_rates,
        health_urls,
        fleet_meta,
        remediation_configs,
    }
}

/// Seed the judge's integrity hash chain from the last stored verdict and
/// verify that verdict's hash matches its content. Returns whether the chain
/// is verified (true) or corrupted (false). Non-fatal failures (empty DB,
/// storage unreachable, pre-chain era) all return true — they are not integrity
/// violations, only absences.
pub async fn seed_integrity_chain(
    storage_port: &Arc<dyn domain::storage::StoragePort>,
    judge: &judge::Judge,
) -> bool {
    match storage_port.list_verdicts(1).await {
        Ok(verdicts) if !verdicts.is_empty() => {
            let last = &verdicts[0];
            let verified = judge::verify_verdict_integrity(last);
            if let Some(ref hash) = last.integrity_hash {
                judge.seed_chain(Some(hash.clone()));
                if verified {
                    klog!(
                        "[Ring 2] Integrity chain seeded + VERIFIED: {}…",
                        &hash[..16.min(hash.len())]
                    );
                    true
                } else {
                    tracing::error!(
                        verdict_id = %last.id,
                        hash = %hash,
                        "INTEGRITY VIOLATION: last verdict hash does not match — chain may be corrupted"
                    );
                    klog!(
                        "[Ring 2] ⚠ INTEGRITY VIOLATION on verdict {} — chain seeded but UNVERIFIED",
                        &last.id[..8.min(last.id.len())]
                    );
                    false
                }
            } else {
                klog!("[Ring 2] Integrity chain: last verdict has no hash (pre-chain era)");
                true // pre-chain verdicts are not a violation
            }
        }
        Ok(_) => {
            klog!("[Ring 2] Integrity chain: no previous verdicts (first boot or empty DB)");
            true // empty DB is not a violation
        }
        Err(e) => {
            klog!(
                "[Ring 2] Integrity chain: failed to load (non-fatal): {}",
                e
            );
            true // DB unreachable at boot = degraded, not integrity violation
        }
    }
}

/// Build fleet probe targets from Dog health URLs (backends.toml SoT).
/// Pure transform: wires `props_url` + `expected_n_ctx` for context-drift
/// detection (D1) and `models_url` for model-identity verification (B4).
pub fn build_fleet_targets(
    health_urls: &HashMap<String, Option<String>>,
    fleet_meta: &HashMap<String, (String, u32, String, Option<String>)>,
) -> Vec<FleetTarget> {
    health_urls
        .iter()
        .filter_map(|(name, url)| {
            url.as_ref().map(|u| {
                let (base_url, ctx_size, model_name, api_key) =
                    fleet_meta.get(name.as_str()).cloned().unwrap_or_default();
                let base_stripped = base_url
                    .trim_end_matches('/')
                    .trim_end_matches("/v1")
                    .to_string();
                // Derive /props URL from base_url (strip /v1 suffix)
                let props_url = if !base_url.is_empty() {
                    Some(base_stripped.clone() + "/props")
                } else {
                    None
                };
                // Derive /v1/models URL for model identity verification (B4)
                let models_url = if !base_url.is_empty() {
                    Some(base_stripped + "/v1/models")
                } else {
                    None
                };
                FleetTarget {
                    dog_name: name.clone(),
                    health_url: u.clone(),
                    props_url,
                    models_url,
                    expected_n_ctx: ctx_size,
                    expected_model: model_name,
                    api_key,
                }
            })
        })
        .collect()
}
