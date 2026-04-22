//! REST handlers for Dog roster — /dogs list + register/heartbeat/deregister lifecycle.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use super::types::{
    AppState, DeregisterResponse, ErrorResponse, HeartbeatResponse, RegisterDogRequest,
    RegisterDogResponse, RegisteredDog,
};

/// GET /dogs — list active Dog IDs.
pub async fn dogs_handler(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    Json(state.judge.load_full().dog_ids())
}

/// POST /dogs/register — register a new Dog at runtime via calibration challenge.
/// Auth-gated. Builds a new Judge with the additional Dog, validates it can score,
/// then atomically swaps the roster via ArcSwap.
pub async fn register_dog_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterDogRequest>,
) -> Result<Json<RegisterDogResponse>, (StatusCode, Json<ErrorResponse>)> {
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
        backend_type: crate::infra::config::BackendType::OpenAi,
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
        prompt_tier: crate::infra::config::PromptTier::Full,
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
            cfg.prompt_tier,
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
            RegisteredDog {
                registered_at: std::time::Instant::now(),
                last_heartbeat: std::time::Instant::now(),
                ttl_secs: 120,
            },
        );
    }
    state.judge.store(Arc::new(new_judge));

    tracing::info!(dog_id = %name, roster_size, "Dog registered via /dogs/register");
    Ok(Json(RegisterDogResponse {
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
) -> Result<Json<HeartbeatResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check dynamically registered Dogs first (they have TTL).
    let mut map = state
        .registered_dogs
        .write()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(entry) = map.get_mut(&dog_id) {
        entry.last_heartbeat = std::time::Instant::now();
        return Ok(Json(HeartbeatResponse {
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
        return Ok(Json(HeartbeatResponse {
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
) -> Result<Json<DeregisterResponse>, (StatusCode, Json<ErrorResponse>)> {
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
        Ok(Json(DeregisterResponse {
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
