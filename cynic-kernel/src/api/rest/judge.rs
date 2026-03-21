//! REST API handlers for judgment — /judge, /verdict/{id}, /verdicts.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use super::types::*;
use super::response::{verdict_to_response, verdict_response_cached};

/// Max content length in chars — caps token consumption per request.
const MAX_CONTENT_LEN: usize = 4_000;
const MAX_CONTEXT_LEN: usize = 2_000;

pub async fn judge_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<JudgeRequest>,
) -> Result<Json<JudgeResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Input validation — prevent token drain attacks
    let content = req.content.trim().to_string();
    if content.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "content must not be empty".into() })));
    }
    if content.len() > MAX_CONTENT_LEN {
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: format!("content exceeds {} chars (got {})", MAX_CONTENT_LEN, content.len()),
        })));
    }
    if let Some(ref ctx) = req.context
        && ctx.len() > MAX_CONTEXT_LEN
    {
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: format!("context exceeds {} chars (got {})", MAX_CONTEXT_LEN, ctx.len()),
        })));
    }
    if let Some(ref domain) = req.domain
        && domain.len() > 64
    {
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "domain exceeds 64 chars".into(),
        })));
    }

    // Shared pipeline: embed → cache → crystals → sessions → evaluate → store → CCM
    let deps = crate::pipeline::PipelineDeps {
        judge: &state.judge,
        storage: state.storage.as_ref(),
        embedding: state.embedding.as_ref(),
        usage: &state.usage,
        verdict_cache: &state.verdict_cache,
    };
    let result = crate::pipeline::run(
        content, req.context, req.domain, req.dogs.as_deref(), &deps,
    ).await.map_err(|e| {
        eprintln!("[REST] Judge error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "evaluation failed".into() }))
    })?;

    match result {
        crate::pipeline::PipelineResult::CacheHit { verdict, similarity } => {
            eprintln!("[REST] Verdict cache HIT (similarity: {:.4})", similarity);
            Ok(Json(verdict_response_cached(&verdict, similarity)))
        }
        crate::pipeline::PipelineResult::Evaluated { verdict } => {
            Ok(Json(verdict_to_response(verdict.as_ref())))
        }
    }
}

pub async fn get_verdict_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<JudgeResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage.get_verdict(&id).await {
        Ok(Some(v)) => Ok(Json(verdict_to_response(&v))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Verdict {} not found", id) }),
        )),
        Err(e) => {
            eprintln!("[REST] verdict/{} error: {}", id, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "storage unavailable".into() })))
        }
    }
}

pub async fn list_verdicts_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<JudgeResponse>>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage.list_verdicts(20).await {
        Ok(verdicts) => Ok(Json(verdicts.iter().map(verdict_to_response).collect())),
        Err(e) => {
            eprintln!("[REST] verdicts error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "storage unavailable".into() })))
        }
    }
}

// ── TESTS ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::domain::ccm;

    #[test]
    fn content_hash_deterministic() {
        let a = ccm::content_hash("hello");
        let b = ccm::content_hash("hello");
        assert_eq!(a, b);
    }

    #[test]
    fn content_hash_different_inputs_differ() {
        assert_ne!(ccm::content_hash("foo"), ccm::content_hash("bar"));
    }
}
