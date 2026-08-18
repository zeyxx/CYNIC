//! Crystal query and CRUD REST handlers.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::api::rest::response::storage_error;
use crate::api::rest::types::{AppState, ErrorResponse};
use crate::domain::ccm::{self, Crystal};

pub fn crystal_to_json(c: &Crystal) -> serde_json::Value {
    serde_json::json!({
        "id": c.id,
        "content": c.content,
        "domain": c.domain,
        "confidence": c.confidence,
        "observations": c.observations,
        "contributing_verdicts": c.contributing_verdicts,
        "state": c.state.to_string(),
        "created_at": c.created_at,
        "updated_at": c.updated_at,
        "certainty": c.certainty,
        "variance_m2": c.variance_m2,
        "mean_quorum": c.mean_quorum,
        "howl_count": c.howl_count,
        "wag_count": c.wag_count,
        "growl_count": c.growl_count,
        "bark_count": c.bark_count,
        "source_diversity": c.source_diversity,
        "relations": c.relations,
        "contributing_sources": c.contributing_sources,
        "shattered_at": c.shattered_at,
        "shatter_reason": c.shatter_reason,
        "shatter_source": c.shatter_source,
    })
}

#[derive(Debug, Deserialize, Default)]
pub struct CrystalsQuery {
    pub limit: Option<u32>,
    pub domain: Option<String>,
    pub state: Option<String>,
}

pub async fn crystals_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CrystalsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(50).min(200);
    match state
        .storage
        .list_crystals_filtered(limit, q.domain.as_deref(), q.state.as_deref())
        .await
    {
        Ok(crystals) => Ok(Json(crystals.iter().map(crystal_to_json).collect())),
        Err(e) => {
            tracing::warn!(error = %e, "crystals list failed");
            Err(storage_error())
        }
    }
}

pub async fn crystal_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage.get_crystal(&id).await {
        Ok(Some(c)) => Ok(Json(crystal_to_json(&c))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Crystal {id} not found"),
            }),
        )),
        Err(e) => {
            tracing::warn!(crystal_id = %id, error = %e, "crystal get failed");
            Err(storage_error())
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateCrystalRequest {
    pub content: String,
    pub domain: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ObserveCrystalHyphaRequest {
    pub score: f64,
    pub source: String,
    pub domain: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub sentiment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ShatterCrystalRequest {
    pub reason: String,
    pub source: String,
}

/// POST /crystals — create a new crystal directly (not via judge pipeline).
pub async fn create_crystal_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateCrystalRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    if req.content.trim().is_empty() || req.content.chars().count() > 2000 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "content must be 1-2000 characters".into(),
            }),
        ));
    }
    let domain = req.domain.unwrap_or_else(|| "general".into());
    let id = format!(
        "{:x}",
        ccm::content_hash(&format!("{}:{}", domain, req.content))
    );
    let now = chrono::Utc::now().timestamp_millis();

    let crystal = Crystal {
        id: id.clone(),
        content: req.content,
        domain: domain.clone(),
        confidence: 0.0,
        observations: 0,
        state: ccm::CrystalState::Forming,
        created_at: now,
        updated_at: now,
        contributing_verdicts: vec![],
        certainty: 0.0,
        variance_m2: 0.0,
        mean_quorum: 0.0,
        howl_count: 0,
        wag_count: 0,
        growl_count: 0,
        bark_count: 0,
        source_diversity: 0,
        relations: std::collections::BTreeMap::new(),
        embedding: None,
        contributing_sources: std::collections::BTreeMap::new(),
        shattered_at: None,
        shatter_reason: None,
        shatter_source: None,
    };
    if let Err(e) = state.storage.store_crystal(&crystal).await {
        tracing::warn!(error = %e, "create crystal failed");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "storage unavailable".into(),
            }),
        ));
    }
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id,
            "domain": domain,
            "state": "forming",
        })),
    ))
}

/// DELETE /crystals/{id} — delete a crystal by ID. Idempotent.
pub async fn delete_crystal_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    if let Err(e) = state.storage.delete_crystal(&id).await {
        tracing::warn!(crystal_id = %id, error = %e, "delete crystal failed");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "storage unavailable".into(),
            }),
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// POST /crystals/{id}/observe — hypha observation (non-verdict source).
pub async fn observe_crystal_hypha_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ObserveCrystalHyphaRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    if req.source.trim().is_empty() || req.source.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "source must be 1-100 characters".into(),
            }),
        ));
    }
    if req.domain.trim().is_empty() || req.domain.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "domain must be 1-100 characters".into(),
            }),
        ));
    }
    if !req.score.is_finite() || req.score < 0.0 || req.score > 1.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "score must be between 0.0 and 1.0".into(),
            }),
        ));
    }
    let content = req.content.as_deref().unwrap_or("");
    if content.chars().count() > 2000 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "content must be <= 2000 characters".into(),
            }),
        ));
    }
    let sentiment = req.sentiment.as_deref();
    if let Some(s) = sentiment
        && !matches!(s, "positive" | "negative" | "neutral")
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "sentiment must be positive, negative, or neutral".into(),
            }),
        ));
    }
    let now = chrono::Utc::now().timestamp_millis();
    match state
        .storage
        .observe_crystal_hypha(
            &id,
            content,
            &req.domain,
            req.score,
            now,
            &req.source,
            sentiment,
        )
        .await
    {
        Ok(()) => Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "crystal_id": id,
                "observed": true,
                "source": req.source,
            })),
        )),
        Err(ref e) if e.to_string().contains("dissolved") => Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "crystal is dissolved — cannot observe".into(),
            }),
        )),
        Err(e) => {
            tracing::warn!(error = %e, "observe_crystal_hypha failed");
            Err(storage_error())
        }
    }
}

/// POST /crystals/{id}/shatter — instant dissolution. Cortex/Internal only.
pub async fn shatter_crystal_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    role: Option<axum::Extension<crate::api::rest::types::Role>>,
    Json(body): Json<ShatterCrystalRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    if role.as_ref().map(|r| &r.0) == Some(&crate::api::rest::types::Role::Organ) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "ORGAN role cannot shatter crystals".into(),
            }),
        ));
    }
    if body.reason.trim().is_empty() || body.reason.len() > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "reason must be 1-500 characters".into(),
            }),
        ));
    }
    let now = chrono::Utc::now().timestamp_millis();
    match state
        .storage
        .shatter_crystal(&id, &body.reason, &body.source, now)
        .await
    {
        Ok(()) => {
            let _ = state
                .event_tx
                .send(crate::domain::events::KernelEvent::CrystalShattered {
                    crystal_id: id.clone(),
                    domain: String::new(),
                    reason: body.reason.clone(),
                });
            Ok((
                StatusCode::OK,
                Json(serde_json::json!({
                    "crystal_id": id,
                    "shattered": true,
                })),
            ))
        }
        Err(e) => {
            tracing::warn!(error = %e, "shatter_crystal failed");
            Err(storage_error())
        }
    }
}
