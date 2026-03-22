//! REST API handlers for data access — /crystals, /crystal/{id}, /usage, crystal CRUD.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};
use crate::domain::ccm;
use crate::domain::ccm::Crystal;
use crate::domain::dog::PHI_INV;

#[derive(Deserialize, Default)]
pub struct CrystalsQuery {
    /// Max results (default 50, max 200)
    pub limit: Option<u32>,
    /// Filter by domain (e.g., "chess", "trading")
    pub domain: Option<String>,
    /// Filter by state (e.g., "crystallized", "canonical", "forming")
    pub state: Option<String>,
}

pub async fn crystals_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CrystalsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(50).min(200);
    // If domain+state filter provided, use domain-specific query; otherwise list all
    match state.storage.list_crystals_filtered(limit, q.domain.as_deref(), q.state.as_deref()).await {
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
            tracing::warn!(error = %e, "crystals list failed");
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
            tracing::warn!(crystal_id = %id, error = %e, "crystal get failed");
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
        tracing::warn!(error = %e, "create crystal failed");
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
        tracing::warn!(crystal_id = %id, error = %e, "delete crystal failed");
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
        tracing::warn!(crystal_id = %id, error = %e, "observe crystal failed");
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: "storage unavailable".into(),
        })));
    }
    Ok(Json(serde_json::json!({ "status": "observed" })))
}

// ── Dark table endpoints ─────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct ObservationsQuery {
    pub limit: Option<u32>,
    pub domain: Option<String>,
    pub agent_id: Option<String>,
}

/// GET /observations — raw workflow observations (10K+ rows, previously invisible).
pub async fn observations_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ObservationsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(100).min(500);
    match state.storage.list_observations_raw(q.domain.as_deref(), q.agent_id.as_deref(), limit).await {
        Ok(rows) => Ok(Json(rows)),
        Err(e) => {
            tracing::warn!(error = %e, "observations list failed");
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "storage unavailable".into() })))
        }
    }
}

/// GET /sessions — session summaries (previously invisible).
pub async fn sessions_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CrystalsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(50).min(200);
    match state.storage.list_session_summaries(limit).await {
        Ok(summaries) => {
            let items: Vec<serde_json::Value> = summaries.iter().map(|s| {
                serde_json::json!({
                    "session_id": s.session_id,
                    "agent_id": s.agent_id,
                    "summary": s.summary,
                    "observations_count": s.observations_count,
                    "created_at": s.created_at,
                })
            }).collect();
            Ok(Json(items))
        }
        Err(e) => {
            tracing::warn!(error = %e, "sessions list failed");
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "storage unavailable".into() })))
        }
    }
}

#[derive(Deserialize, Default)]
pub struct AuditQuery {
    pub limit: Option<u32>,
    pub tool: Option<String>,
    pub agent_id: Option<String>,
}

/// GET /audit — MCP audit trail (previously MCP-only, 11K+ rows).
pub async fn audit_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(50).min(100);
    match state.coord.query_audit(q.tool.as_deref(), q.agent_id.as_deref(), limit).await {
        Ok(rows) => Ok(Json(rows)),
        Err(e) => {
            tracing::warn!(error = %e, "audit query failed");
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "coordination unavailable".into() })))
        }
    }
}
