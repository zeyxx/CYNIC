//! REST API handlers for data access — /crystals, /crystal/{id}, /usage, crystal CRUD.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};
use crate::domain::ccm;
use crate::domain::ccm::Crystal;
use crate::domain::dog::PHI_INV;

pub async fn crystals_handler(
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
        Err(e) => {
            eprintln!("[REST] crystals error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "storage unavailable".into() })))
        }
    }
}

pub async fn crystal_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage.get_crystal(&id).await {
        Ok(Some(c)) => Ok(Json(serde_json::json!({
            "id": c.id,
            "content": c.content,
            "domain": c.domain,
            "confidence": c.confidence,
            "observations": c.observations,
            "state": format!("{:?}", c.state),
            "created_at": c.created_at,
            "updated_at": c.updated_at,
        }))),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ErrorResponse { error: format!("Crystal {} not found", id) }))),
        Err(e) => {
            eprintln!("[REST] crystal/{} error: {}", id, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "storage unavailable".into() })))
        }
    }
}

pub async fn usage_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let usage = state.usage.lock().await;
    let merged = usage.merged_dogs();
    let mut dogs: Vec<serde_json::Value> = merged.iter().map(|(id, d)| {
        let avg_latency = d.total_latency_ms.checked_div(d.requests).unwrap_or(0);
        serde_json::json!({
            "dog_id": id,
            "prompt_tokens": d.prompt_tokens,
            "completion_tokens": d.completion_tokens,
            "total_tokens": d.total_tokens(),
            "requests": d.requests,
            "failures": d.failures,
            "avg_latency_ms": avg_latency,
        })
    }).collect();
    dogs.sort_by(|a, b| b["requests"].as_u64().cmp(&a["requests"].as_u64()));
    Json(serde_json::json!({
        "total_tokens": usage.total_tokens(),
        "total_requests": usage.all_time_requests(),
        "estimated_cost_usd": usage.estimated_cost_usd(),
        "uptime_seconds": usage.uptime_seconds(),
        "per_dog": dogs,
    }))
}

// ── Crystal CRUD ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateCrystalRequest {
    pub content: String,
    pub domain: Option<String>,
}

/// POST /crystal — create a new crystal directly (not via judge pipeline).
/// Returns 201 with the crystal ID, domain, and initial state.
pub async fn create_crystal_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateCrystalRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    if req.content.trim().is_empty() || req.content.len() > 2000 {
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "content must be 1-2000 characters".into(),
        })));
    }
    let domain = req.domain.unwrap_or_else(|| "general".into());
    let id = format!("{:x}", ccm::content_hash(&format!("{}:{}", domain, req.content)));
    let now = chrono::Utc::now().to_rfc3339();

    let crystal = Crystal {
        id: id.clone(),
        content: req.content,
        domain: domain.clone(),
        confidence: 0.0,
        observations: 0,
        state: ccm::CrystalState::Forming,
        created_at: now.clone(),
        updated_at: now,
    };
    if let Err(e) = state.storage.store_crystal(&crystal).await {
        eprintln!("[REST] create crystal error: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: "storage unavailable".into(),
        })));
    }
    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "id": id,
        "domain": domain,
        "state": "Forming",
    }))))
}

/// DELETE /crystal/{id} — delete a crystal by ID. Idempotent.
pub async fn delete_crystal_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    if let Err(e) = state.storage.delete_crystal(&id).await {
        eprintln!("[REST] delete crystal/{} error: {}", id, e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: "storage unavailable".into(),
        })));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ObserveCrystalRequest {
    pub content: String,
    pub domain: Option<String>,
    pub score: Option<f64>,
}

/// POST /crystal/{id}/observe — observe a score for an existing crystal.
pub async fn observe_crystal_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ObserveCrystalRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let domain = req.domain.unwrap_or_else(|| "general".into());
    let score = req.score.unwrap_or(0.5);
    // Normalize score to confidence range (same as pipeline)
    let confidence = (score / PHI_INV).min(1.0);
    let now = chrono::Utc::now().to_rfc3339();

    if let Err(e) = state.storage.observe_crystal(&id, &req.content, &domain, confidence, &now).await {
        eprintln!("[REST] observe crystal/{} error: {}", id, e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: "storage unavailable".into(),
        })));
    }
    Ok(Json(serde_json::json!({ "status": "observed" })))
}
