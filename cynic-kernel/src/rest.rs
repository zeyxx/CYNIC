//! REST API — JSON interface for external clients (React, curl, etc.)
//! Runs alongside gRPC on a separate port.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use chrono;

use crate::dog::{Verdict, PHI_INV};
use crate::judge::Judge;
use crate::storage_port::StoragePort;
use crate::ccm;

// ── SHARED STATE ───────────────────────────────────────────

pub struct AppState {
    pub judge: Judge,
    pub storage: Arc<dyn StoragePort>,
}

// ── REQUEST / RESPONSE TYPES ───────────────────────────────

#[derive(Deserialize)]
pub struct JudgeRequest {
    pub content: String,
    pub context: Option<String>,
    pub domain: Option<String>,
    /// Optional: evaluate with only these Dogs (by ID). If omitted, all Dogs are used.
    pub dogs: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct JudgeResponse {
    pub verdict_id: String,
    pub verdict: String,
    pub q_score: QScoreResponse,
    pub reasoning: ReasoningResponse,
    pub dogs_used: String,
    pub phi_max: f64,
    pub dog_scores: Vec<DogScoreResponse>,
    pub anomaly_detected: bool,
    pub max_disagreement: f64,
    pub anomaly_axiom: Option<String>,
}

#[derive(Serialize)]
pub struct DogScoreResponse {
    pub dog_id: String,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
    pub reasoning: ReasoningResponse,
}

#[derive(Serialize)]
pub struct QScoreResponse {
    pub total: f64,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
}

#[derive(Serialize)]
pub struct ReasoningResponse {
    pub fidelity: String,
    pub phi: String,
    pub verify: String,
    pub culture: String,
    pub burn: String,
    pub sovereignty: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub phi_max: f64,
    pub axioms: Vec<String>,
    pub dogs: Vec<DogHealthResponse>,
}

#[derive(Serialize)]
pub struct DogHealthResponse {
    pub id: String,
    pub kind: String,
}

// ── ROUTER ─────────────────────────────────────────────────

pub fn router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/judge", post(judge_handler))
        .route("/dogs", get(dogs_handler))
        .route("/crystals", get(crystals_handler))
        .route("/verdict/{id}", get(get_verdict_handler))
        .route("/verdicts", get(list_verdicts_handler))
        .route("/health", get(health_handler))
        .fallback_service(ServeDir::new("static"))
        .layer(cors)
        .with_state(state)
}

// ── HANDLERS ───────────────────────────────────────────────

async fn judge_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<JudgeRequest>,
) -> Result<Json<JudgeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let stimulus = crate::dog::Stimulus {
        content: req.content,
        context: req.context,
        domain: req.domain,
    };

    let verdict = state.judge.evaluate(&stimulus, req.dogs.as_deref()).await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string() }),
        ))?;

    // Store verdict (best effort — don't fail the request if storage is down)
    if let Err(e) = state.storage.store_verdict(&verdict).await {
        eprintln!("[REST] Warning: failed to store verdict: {}", e);
    }

    // CCM: observe crystal (best effort — learning loop)
    {
        let crystal_id = format!("{:x}", md5_hash(&verdict.stimulus_summary));
        let domain = stimulus.domain.unwrap_or_else(|| "general".to_string());
        let now = chrono::Utc::now().to_rfc3339();
        let existing = state.storage.get_crystal(&crystal_id).await.ok().flatten();
        let crystal = match existing {
            Some(c) => ccm::update_crystal(&c, verdict.q_score.total, &now),
            None => ccm::new_crystal(crystal_id, verdict.stimulus_summary.clone(), domain, verdict.q_score.total, &now),
        };
        if let Err(e) = state.storage.store_crystal(&crystal).await {
            eprintln!("[REST] Warning: failed to store crystal: {}", e);
        } else {
            eprintln!("[CCM] Crystal '{}' → {:?} (obs: {}, conf: {:.3})",
                crystal.content.chars().take(40).collect::<String>(),
                crystal.state, crystal.observations, crystal.confidence);
        }
    }

    Ok(Json(verdict_to_response(&verdict)))
}

async fn get_verdict_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<JudgeResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage.get_verdict(&id).await {
        Ok(Some(v)) => Ok(Json(verdict_to_response(&v))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Verdict {} not found", id) }),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string() }),
        )),
    }
}

async fn list_verdicts_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<JudgeResponse>>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage.list_verdicts(20).await {
        Ok(verdicts) => Ok(Json(verdicts.iter().map(verdict_to_response).collect())),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string() }),
        )),
    }
}

async fn crystals_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage.list_crystals(20).await {
        Ok(crystals) => {
            let items: Vec<serde_json::Value> = crystals.iter().map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "content": c.content,
                    "domain": c.domain,
                    "confidence": c.confidence,
                    "observations": c.observations,
                    "state": format!("{:?}", c.state),
                    "created_at": c.created_at,
                    "updated_at": c.updated_at,
                })
            }).collect();
            Ok(Json(items))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

async fn dogs_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<String>> {
    Json(state.judge.dog_ids())
}

async fn health_handler(
    State(state): State<Arc<AppState>>,
) -> Json<HealthResponse> {
    let dog_ids = state.judge.dog_ids();
    let dogs: Vec<DogHealthResponse> = dog_ids.into_iter().map(|id| {
        let kind = if id == "deterministic-dog" {
            "heuristic"
        } else {
            "inference"
        }.to_string();
        DogHealthResponse { id, kind }
    }).collect();

    let status = if dogs.is_empty() {
        "critical"
    } else if dogs.len() == 1 {
        "degraded"
    } else {
        "sovereign"
    }.to_string();

    Json(HealthResponse {
        status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        phi_max: PHI_INV,
        axioms: vec![
            "FIDELITY".into(),
            "PHI".into(),
            "VERIFY/FALSIFY".into(),
            "CULTURE".into(),
            "BURN".into(),
            "SOVEREIGNTY".into(),
        ],
        dogs,
    })
}

// ── HELPERS ────────────────────────────────────────────────

/// Simple hash for crystal IDs. Not cryptographic — just deterministic content addressing.
fn md5_hash(input: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    for byte in input.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3); // FNV-1a prime
    }
    h
}

fn verdict_to_response(v: &Verdict) -> JudgeResponse {
    JudgeResponse {
        verdict_id: v.id.clone(),
        verdict: format!("{:?}", v.kind),
        q_score: QScoreResponse {
            total: v.q_score.total,
            fidelity: v.q_score.fidelity,
            phi: v.q_score.phi,
            verify: v.q_score.verify,
            culture: v.q_score.culture,
            burn: v.q_score.burn,
            sovereignty: v.q_score.sovereignty,
        },
        reasoning: ReasoningResponse {
            fidelity: v.reasoning.fidelity.clone(),
            phi: v.reasoning.phi.clone(),
            verify: v.reasoning.verify.clone(),
            culture: v.reasoning.culture.clone(),
            burn: v.reasoning.burn.clone(),
            sovereignty: v.reasoning.sovereignty.clone(),
        },
        dogs_used: v.dog_id.clone(),
        phi_max: PHI_INV,
        dog_scores: v.dog_scores.iter().map(|ds| DogScoreResponse {
            dog_id: ds.dog_id.clone(),
            fidelity: ds.fidelity,
            phi: ds.phi,
            verify: ds.verify,
            culture: ds.culture,
            burn: ds.burn,
            sovereignty: ds.sovereignty,
            reasoning: ReasoningResponse {
                fidelity: ds.reasoning.fidelity.clone(),
                phi: ds.reasoning.phi.clone(),
                verify: ds.reasoning.verify.clone(),
                culture: ds.reasoning.culture.clone(),
                burn: ds.reasoning.burn.clone(),
                sovereignty: ds.reasoning.sovereignty.clone(),
            },
        }).collect(),
        anomaly_detected: v.anomaly_detected,
        max_disagreement: v.max_disagreement,
        anomaly_axiom: v.anomaly_axiom.clone(),
    }
}
