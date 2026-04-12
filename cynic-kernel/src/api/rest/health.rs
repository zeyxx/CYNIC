//! REST API handlers for health and introspection — /health, /dogs, /agents.

use axum::{
    extract::{Path, Request, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use super::response::coordination_error;
use super::types::{AppState, DogHealthResponse, ErrorResponse};
use crate::domain::dog::{AXIOM_NAMES, PHI_INV};
use crate::domain::health_gate::{count_healthy_dogs, system_health_assessment_with_contract};

pub async fn dogs_handler(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    Json(state.judge.load_full().dog_ids())
}

/// GET /live — Liveness probe. Returns 200 if the process is running.
/// No dependencies checked — this is "is the kernel alive?" for systemd/k8s.
pub async fn liveness_handler() -> StatusCode {
    StatusCode::OK
}

/// GET /ready — Readiness probe. Returns 200 if the kernel can serve requests.
/// F22: Caches DB ping result (30s TTL) to avoid hammering storage on every probe.
/// Dog health is O(1) (reads circuit breaker state) — no caching needed.
pub async fn readiness_handler(State(state): State<Arc<AppState>>) -> StatusCode {
    let judge = state.judge.load_full();
    let dog_health = judge.dog_health();
    let (healthy_dogs, total_dogs) = count_healthy_dogs(&dog_health);
    let storage_ok = match state.ready_cache.get() {
        Some(cached) => cached,
        None => {
            let ok = state.storage.ping().await.is_ok();
            state.ready_cache.set(ok);
            ok
        }
    };
    let live_dog_ids = judge.dog_ids();
    let contract_delta = state.system_contract.assess(&live_dog_ids);
    let probes_degraded =
        crate::domain::probe::EnvironmentSnapshot::is_degraded(&state.environment);
    let stale_tasks = state.task_health.readiness_stale_tasks();
    let assessment = system_health_assessment_with_contract(
        healthy_dogs,
        total_dogs,
        storage_ok,
        probes_degraded,
        &stale_tasks,
        Some(&contract_delta),
    );
    if assessment.healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

pub async fn health_handler(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> (StatusCode, Json<serde_json::Value>) {
    // Check if caller has valid auth — return full details only if authenticated.
    // Uses constant_time_eq to prevent timing oracle (same as auth_middleware).
    let authenticated = match &state.api_key {
        Some(key) => request
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .is_some_and(|t| super::middleware::constant_time_eq(t.as_bytes(), key.as_bytes())),
        None => true, // No auth configured → everyone gets full details
    };

    let judge = state.judge.load_full();
    let dog_health = judge.dog_health();
    let (healthy_dogs, total_dogs) = count_healthy_dogs(&dog_health);

    let storage_ok = state.storage.ping().await.is_ok();

    // Self-model: compare contract against live roster
    let live_dog_ids = judge.dog_ids();
    let contract_delta = state.system_contract.assess(&live_dog_ids);

    let probes_degraded =
        crate::domain::probe::EnvironmentSnapshot::is_degraded(&state.environment);
    let stale_tasks = state.task_health.readiness_stale_tasks();
    let readiness = system_health_assessment_with_contract(
        healthy_dogs,
        total_dogs,
        storage_ok,
        probes_degraded,
        &stale_tasks,
        Some(&contract_delta),
    );
    let http_code = if readiness.healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    // Public: minimal info only — dog_count withheld to prevent attack surface mapping
    // HTTP status code tells the story: 200 = healthy, 503 = degraded/critical.
    // Any monitoring tool can check without parsing JSON: curl -sf URL || alert
    if !authenticated {
        return (
            http_code,
            Json(serde_json::json!({
                "status": readiness.status,
                "version": env!("CYNIC_VERSION"),
                "phi_max": PHI_INV,
            })),
        );
    }

    // Authenticated: full details
    let dogs: Vec<DogHealthResponse> = dog_health
        .into_iter()
        .map(|(id, circuit, failures)| {
            let kind = if id == "deterministic-dog" {
                "heuristic"
            } else {
                "inference"
            }
            .to_string();
            DogHealthResponse {
                id,
                kind,
                circuit,
                failures,
            }
        })
        .collect();

    let organ_quality: Vec<serde_json::Value> = judge
        .dog_quality_snapshot()
        .into_iter()
        .map(|(id, stats)| {
            serde_json::json!({
                "dog": id,
                "json_valid_rate": stats.json_valid_rate(),
                "capability_limit_rate": stats.capability_limit_rate(),
                "total_calls": stats.total_calls,
            })
        })
        .collect();

    let (total_requests, total_tokens, estimated_cost_usd, uptime_seconds) = {
        let usage = state.usage.lock().await;
        (
            usage.all_time_requests(),
            usage.total_tokens(),
            usage.estimated_cost_usd(),
            usage.uptime_seconds(),
        )
    };

    // Proprioception: crystal state summary (best-effort, non-blocking)
    let crystal_summary = match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        state.storage.list_crystals(200),
    )
    .await
    {
        Ok(Ok(crystals)) => {
            use crate::domain::ccm::CrystalState;
            let (mut forming, mut crystallized, mut canonical, mut decaying) =
                (0u32, 0u32, 0u32, 0u32);
            for c in &crystals {
                match c.state {
                    CrystalState::Forming => forming += 1,
                    CrystalState::Crystallized => crystallized += 1,
                    CrystalState::Canonical => canonical += 1,
                    CrystalState::Decaying => decaying += 1,
                    CrystalState::Dissolved => {}
                }
            }
            serde_json::json!({
                "total": crystals.len(),
                "forming": forming,
                "crystallized": crystallized,
                "canonical": canonical,
                "decaying": decaying,
                "loop_active": crystallized + canonical > 0,
            })
        }
        _ => serde_json::json!({ "error": "unavailable" }),
    };

    (
        http_code,
        Json(serde_json::json!({
            "status": readiness.status,
            "version": env!("CYNIC_VERSION"),
            "phi_max": PHI_INV,
            "axioms": AXIOM_NAMES,
            "dogs": dogs,
            "storage": if storage_ok { "connected" } else { "down" },
            "storage_namespace": state.storage_info.namespace,
            "storage_database": state.storage_info.database,
            "storage_metrics": state.storage_metrics(),
            "embedding": if tokio::time::timeout(std::time::Duration::from_secs(2), state.embedding.embed("h")).await.map(|r| r.is_ok()).unwrap_or(false) { "sovereign" } else { "unavailable" },
            "crystals": crystal_summary,
            "organ_quality": organ_quality,
            "contract": {
                "expected_dogs": state.system_contract.expected_dogs(),
                "expected_count": state.system_contract.expected_count(),
                "missing_dogs": &contract_delta.missing,
                "unexpected_dogs": &contract_delta.unexpected,
                "fulfilled": contract_delta.fulfilled,
            },
            "readiness": {
                "status": readiness.status,
                "healthy": readiness.healthy,
                "healthy_dogs": healthy_dogs,
                "total_dogs": total_dogs,
                "storage_ok": storage_ok,
                "probes_degraded": probes_degraded,
                "stale_tasks": stale_tasks,
                "causes": readiness.causes,
            },
            "environment": state.environment.read().ok().and_then(|e| e.clone()),
            "chain_verified": state.chain_verified.load(std::sync::atomic::Ordering::Relaxed),
            "verdict_cache_size": state.verdict_cache.len(),
            "background_tasks": state.task_health.snapshot(),
            "total_requests": total_requests,
            "total_tokens": total_tokens,
            "estimated_cost_usd": estimated_cost_usd,
            "uptime_seconds": uptime_seconds,
            "alerts": state.introspection_alerts.read()
                .map(|a| a.clone())
                .unwrap_or_else(|e| {
                    tracing::warn!(error = %e, "introspection_alerts RwLock poisoned");
                    Vec::new()
                }),
        })),
    )
}

/// GET /agents — show active agent sessions and their claims (requires auth)
pub async fn agents_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.coord.who(None).await {
        Ok(snapshot) => Ok(Json(
            serde_json::to_value(snapshot.into_summary()).unwrap_or_default(),
        )),
        Err(e) => {
            tracing::warn!(error = %e, "agents query failed");
            Err(coordination_error())
        }
    }
}

/// GET /metrics — Prometheus text exposition format.
/// Public endpoint (no auth) — metrics are operational data, not secrets.
pub async fn metrics_handler(
    State(state): State<Arc<AppState>>,
) -> (
    StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    String,
) {
    let mut out = state.metrics.render_prometheus();

    // Verdict cache size (gauge)
    {
        use std::fmt::Write;
        let _ = writeln!(
            out,
            "# HELP cynic_verdict_cache_size Current verdict cache entries"
        );
        let _ = writeln!(out, "# TYPE cynic_verdict_cache_size gauge");
        let _ = writeln!(
            out,
            "cynic_verdict_cache_size {}",
            state.verdict_cache.len()
        );
    }

    // Per-dog metrics from usage tracker
    {
        let usage = state.usage.lock().await;
        let merged = usage.merged_dogs();
        let mut dog_data: Vec<(String, u64, u64, u64, u64)> = merged
            .into_iter()
            .map(|(id, u)| {
                (
                    id,
                    u.requests,
                    u.failures,
                    u.total_latency_ms,
                    u.total_tokens(),
                )
            })
            .collect();
        dog_data.sort_by(|a, b| a.0.cmp(&b.0));

        let judge = state.judge.load_full();
        let circuit_states = judge.dog_health();
        crate::domain::metrics::append_dog_metrics(&mut out, &dog_data, &circuit_states);
    }

    // Organ quality metrics
    {
        let judge = state.judge.load_full();
        let snapshots = judge.dog_quality_snapshot();
        crate::domain::metrics::append_organ_metrics(&mut out, &snapshots);
    }

    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        out,
    )
}

// Logic tests live in domain::health_gate::tests — single source of truth.
// This handler only maps (status, is_healthy) → HTTP status code.

/// POST /dogs/register — register a new Dog at runtime via calibration challenge.
/// Auth-gated. Builds a new Judge with the additional Dog, validates it can score,
/// then atomically swaps the roster via ArcSwap.
pub async fn register_dog_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<super::types::RegisterDogRequest>,
) -> Result<Json<super::types::RegisterDogResponse>, (StatusCode, Json<ErrorResponse>)> {
    let name = req.name.trim().to_string();
    if name.is_empty() || name.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "name must be 1-64 characters".into(),
            }),
        ));
    }

    let current = state.judge.load_full();
    if current.dog_ids().iter().any(|id| id == &name) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!("Dog '{name}' already exists in roster"),
            }),
        ));
    }

    let cfg = crate::infra::config::BackendConfig {
        name: name.clone(),
        base_url: req.base_url,
        model: req.model,
        api_key: req.api_key,
        context_size: req.context_size,
        timeout_secs: req.timeout_secs,
        auth_style: crate::infra::config::AuthStyle::Bearer,
        max_tokens: 4096,
        temperature: 0.3,
        disable_thinking: false,
        json_mode: false,
        cost_input_per_mtok: 0.0,
        cost_output_per_mtok: 0.0,
        health_url: None,
        remediation: None,
    };

    let backend = match crate::backends::openai::OpenAiCompatBackend::new(cfg.clone()) {
        Ok(b) => Arc::new(b),
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("backend init failed: {e}"),
                }),
            ));
        }
    };

    let new_dog: Arc<dyn crate::domain::dog::Dog> =
        Arc::new(crate::dogs::inference::InferenceDog::new(
            backend,
            name.clone(),
            cfg.context_size,
            cfg.timeout_secs,
        ));

    // Calibration challenge: evaluate a known stimulus, validate_scores() must pass
    let calibration_stimulus = crate::domain::dog::Stimulus {
        content: "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 — Ruy Lopez opening in chess.".into(),
        context: None,
        domain: Some("chess".into()),
        request_id: None,
    };

    let scores = tokio::time::timeout(
        std::time::Duration::from_secs(cfg.timeout_secs + 5),
        new_dog.evaluate(&calibration_stimulus),
    )
    .await
    .map_err(|_elapsed| {
        (
            StatusCode::GATEWAY_TIMEOUT,
            Json(ErrorResponse {
                error: format!("calibration timed out ({}s)", cfg.timeout_secs + 5),
            }),
        )
    })?
    .map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: format!("calibration failed: {e}"),
            }),
        )
    })?;

    crate::domain::dog::validate_scores(&scores).map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: format!("calibration rejected: {e}"),
            }),
        )
    })?;

    // Build new Judge: clone existing Arcs (refcount++) + add new Dog
    let mut dogs: Vec<Arc<dyn crate::domain::dog::Dog>> =
        current.dogs().iter().map(Arc::clone).collect();
    let mut breakers: Vec<Arc<dyn crate::domain::health_gate::HealthGate>> =
        current.breakers().iter().map(Arc::clone).collect();
    let mut handles: Vec<Option<crate::organ::BackendHandle>> = current.organ_handles().to_vec();

    let new_breaker = Arc::new(crate::infra::circuit_breaker::CircuitBreaker::new(
        name.clone(),
    )) as Arc<dyn crate::domain::health_gate::HealthGate>;

    dogs.push(new_dog);
    breakers.push(new_breaker);
    handles.push(None);

    let new_judge = crate::judge::Judge::new(dogs, breakers).with_organ_handles(handles);
    let chain_hash = current.last_hash_snapshot();
    new_judge.seed_chain(chain_hash);

    let roster_size = new_judge.dog_ids().len();
    // Track TTL for this dynamically registered Dog
    {
        let mut map = state
            .registered_dogs
            .write()
            .unwrap_or_else(|e| e.into_inner());
        map.insert(
            name.clone(),
            super::types::RegisteredDog {
                registered_at: std::time::Instant::now(),
                last_heartbeat: std::time::Instant::now(),
                ttl_secs: 120,
            },
        );
    }
    state.judge.store(Arc::new(new_judge));

    tracing::info!(dog_id = %name, roster_size, "Dog registered via /dogs/register");
    Ok(Json(super::types::RegisterDogResponse {
        dog_id: name,
        calibration: "passed".into(),
        roster_size,
    }))
}

/// POST /dogs/{id}/heartbeat — refresh TTL for a registered Dog.
/// Accepts both dynamically registered Dogs (TTL-managed) and config-based Dogs (static).
pub async fn heartbeat_handler(
    State(state): State<Arc<AppState>>,
    Path(dog_id): Path<String>,
) -> Result<Json<super::types::HeartbeatResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check dynamically registered Dogs first (they have TTL).
    let mut map = state
        .registered_dogs
        .write()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(entry) = map.get_mut(&dog_id) {
        entry.last_heartbeat = std::time::Instant::now();
        return Ok(Json(super::types::HeartbeatResponse {
            dog_id,
            status: "alive".into(),
            ttl_remaining_secs: entry.ttl_secs,
        }));
    }
    drop(map); // Release write lock before querying judge

    // Fall back: check if Dog exists in judge's config-based roster.
    let judge = state.judge.load_full();
    if judge.dog_ids().contains(&dog_id) {
        // Config-based Dogs are permanent (no TTL). Return OK with large TTL value.
        return Ok(Json(super::types::HeartbeatResponse {
            dog_id,
            status: "alive".into(),
            ttl_remaining_secs: 86400, // 24h placeholder — config Dogs don't expire
        }));
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: format!("Dog '{dog_id}' not registered — re-register required"),
        }),
    ))
}

/// DELETE /dogs/{id} — remove a dynamically registered Dog.
pub async fn deregister_handler(
    State(state): State<Arc<AppState>>,
    Path(dog_id): Path<String>,
) -> Result<Json<super::types::DeregisterResponse>, (StatusCode, Json<ErrorResponse>)> {
    {
        let map = state
            .registered_dogs
            .read()
            .unwrap_or_else(|e| e.into_inner());
        if !map.contains_key(&dog_id) {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Dog '{dog_id}' not found in registered Dogs"),
                }),
            ));
        }
    }

    {
        let mut map = state
            .registered_dogs
            .write()
            .unwrap_or_else(|e| e.into_inner());
        map.remove(&dog_id);
    }

    let current = state.judge.load_full();
    if let Some(new_judge) = crate::judge::Judge::without_dog(&current, &dog_id) {
        let roster_size = new_judge.dog_ids().len();
        state.judge.store(Arc::new(new_judge));
        tracing::info!(dog_id = %dog_id, roster_size, "Dog deregistered");
        let _ = state
            .event_tx
            .send(crate::domain::events::KernelEvent::DogExpired {
                dog_id: dog_id.clone(),
            });
        Ok(Json(super::types::DeregisterResponse {
            dog_id,
            status: "deregistered".into(),
            roster_size,
        }))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Dog '{dog_id}' not in roster"),
            }),
        ))
    }
}
