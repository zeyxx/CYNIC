//! REST API handlers for judgment — /judge, /verdict/{id}, /verdicts.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use super::types::*;
use super::response::verdict_to_response;
use crate::domain::ccm;

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

    // CCM feedback: enrich context with crystallized wisdom from this domain
    let domain_hint = req.domain.as_deref().unwrap_or("general");
    let enriched_context = match state.storage.list_crystals(50).await {
        Ok(crystals) => {
            let crystal_ctx = ccm::format_crystal_context(&crystals, domain_hint, 800);
            match (req.context, crystal_ctx) {
                (Some(ctx), Some(cc)) => Some(format!("{}\n\n{}", ctx, cc)),
                (Some(ctx), None) => Some(ctx),
                (None, Some(cc)) => Some(cc),
                (None, None) => None,
            }
        }
        Err(_) => req.context, // Storage down — proceed without crystals
    };

    let stimulus = crate::domain::dog::Stimulus {
        content,
        context: enriched_context,
        domain: req.domain,
    };

    let verdict = state.judge.evaluate(&stimulus, req.dogs.as_deref()).await
        .map_err(|e| {
            eprintln!("[REST] Judge error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "evaluation failed".into() }))
        })?;

    // Store verdict (best effort — don't fail the request if storage is down)
    if let Err(e) = state.storage.store_verdict(&verdict).await {
        eprintln!("[REST] Warning: failed to store verdict: {}", e);
    }

    // Track token usage per Dog
    {
        let mut usage = state.usage.lock().unwrap_or_else(|e| e.into_inner());
        usage.total_requests += 1;
        for ds in &verdict.dog_scores {
            usage.record(&ds.dog_id, ds.prompt_tokens, ds.completion_tokens, ds.latency_ms);
        }
    }

    // CCM: observe crystal — hash the summary (not full content) to deduplicate
    {
        let crystal_id = format!("{:x}", crate::domain::ccm::content_hash(&format!("{}:{}", stimulus.domain.as_deref().unwrap_or("general"), verdict.stimulus_summary)));
        let domain = stimulus.domain.unwrap_or_else(|| "general".to_string());
        let now = chrono::Utc::now().to_rfc3339();
        if let Err(e) = state.storage.observe_crystal(
            &crystal_id, &verdict.stimulus_summary, &domain, verdict.q_score.total, &now
        ).await {
            eprintln!("[REST] Warning: failed to observe crystal: {}", e);
        }
    }

    Ok(Json(verdict_to_response(&verdict)))
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
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string() }),
        )),
    }
}

pub async fn list_verdicts_handler(
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
