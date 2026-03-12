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

use crate::dog::{Verdict, PHI_INV};
use crate::judge::Judge;
use crate::storage_port::StoragePort;

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
    pub reasoning: ReasoningResponse,
}

#[derive(Serialize)]
pub struct QScoreResponse {
    pub total: f64,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
}

#[derive(Serialize)]
pub struct ReasoningResponse {
    pub fidelity: String,
    pub phi: String,
    pub verify: String,
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
}

// ── ROUTER ─────────────────────────────────────────────────

pub fn router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/judge", post(judge_handler))
        .route("/verdict/{id}", get(get_verdict_handler))
        .route("/verdicts", get(list_verdicts_handler))
        .route("/health", get(health_handler))
        .nest_service("/", ServeDir::new("static"))
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

    let verdict = state.judge.evaluate(&stimulus).await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string() }),
        ))?;

    // Store verdict (best effort — don't fail the request if storage is down)
    if let Err(e) = state.storage.store_verdict(&verdict).await {
        eprintln!("[REST] Warning: failed to store verdict: {}", e);
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

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "sovereign".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        phi_max: PHI_INV,
        axioms: vec![
            "FIDELITY".into(),
            "PHI".into(),
            "VERIFY/FALSIFY".into(),
        ],
    })
}

// ── HELPERS ────────────────────────────────────────────────

fn verdict_to_response(v: &Verdict) -> JudgeResponse {
    JudgeResponse {
        verdict_id: v.id.clone(),
        verdict: format!("{:?}", v.kind),
        q_score: QScoreResponse {
            total: v.q_score.total,
            fidelity: v.q_score.fidelity,
            phi: v.q_score.phi,
            verify: v.q_score.verify,
        },
        reasoning: ReasoningResponse {
            fidelity: v.reasoning.fidelity.clone(),
            phi: v.reasoning.phi.clone(),
            verify: v.reasoning.verify.clone(),
        },
        dogs_used: v.dog_id.clone(),
        phi_max: PHI_INV,
        dog_scores: v.dog_scores.iter().map(|ds| DogScoreResponse {
            dog_id: ds.dog_id.clone(),
            fidelity: ds.fidelity,
            phi: ds.phi,
            verify: ds.verify,
            reasoning: ReasoningResponse {
                fidelity: ds.reasoning.fidelity.clone(),
                phi: ds.reasoning.phi.clone(),
                verify: ds.reasoning.verify.clone(),
            },
        }).collect(),
        anomaly_detected: v.anomaly_detected,
        max_disagreement: v.max_disagreement,
        anomaly_axiom: v.anomaly_axiom.clone(),
    }
}
