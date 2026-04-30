//! REST API handlers for judgment — /judge, /verdict/{id}, /verdicts.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use super::response::{storage_error, verdict_response_cached, verdict_to_response};
use super::types::*;

use crate::domain::constants::{MAX_CONTENT_LEN, MAX_CONTEXT_LEN};
use crate::domain::dispatch::Domain;

#[derive(Debug)]
pub(super) struct ValidatedJudgeRequest {
    pub content: String,
    pub context: Option<String>,
    pub domain: Option<String>,
    pub dogs: Option<Vec<String>>,
    pub crystals: bool,
}

pub(super) fn validate_judge_request(
    req: JudgeRequest,
) -> Result<ValidatedJudgeRequest, (StatusCode, Json<ErrorResponse>)> {
    let content = req.content.trim().to_string();
    if content.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "content must not be empty".into(),
            }),
        ));
    }
    let content_chars = content.chars().count();
    if content_chars > MAX_CONTENT_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("content exceeds {MAX_CONTENT_LEN} chars (got {content_chars})"),
            }),
        ));
    }
    if let Some(ref ctx) = req.context
        && ctx.chars().count() > MAX_CONTEXT_LEN
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!(
                    "context exceeds {MAX_CONTEXT_LEN} chars (got {})",
                    ctx.chars().count()
                ),
            }),
        ));
    }
    if let Some(ref domain) = req.domain
        && domain.len() > 64
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "domain exceeds 64 chars".into(),
            }),
        ));
    }

    Ok(ValidatedJudgeRequest {
        content,
        context: req.context,
        domain: req.domain,
        dogs: req.dogs,
        crystals: req.crystals,
    })
}

pub async fn judge_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<JudgeRequest>,
) -> Result<Json<JudgeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let req = validate_judge_request(req)?;

    // Shared pipeline: embed → cache → crystals → sessions → evaluate → store → CCM
    let judge = state.judge.load_full();

    // Domain-aware Dog routing (K15 pattern):
    // 1. If dogs explicitly specified → use those (highest priority)
    // 2. If domain specified → route to Dogs for that domain (K15 consumer pattern)
    // 3. If "sovereign" preset → use sovereign Dogs (backwards compat)
    // 4. Otherwise → use all Dogs (default)
    let dogs = match req.dogs {
        Some(ref d) if d.len() == 1 && d[0] == "sovereign" => {
            // Preset: "sovereign" → live sovereign Dog IDs
            Some(judge.sovereign_dog_ids())
        }
        Some(d) => {
            // Explicit dog list provided → use it
            Some(d)
        }
        None => {
            // No explicit dogs; route by domain if provided
            if let Some(ref domain_str) = req.domain {
                let domain = Domain::parse(Some(domain_str.as_str()));
                let dog_set = domain.dog_set();
                if !dog_set.dogs.is_empty() {
                    Some(dog_set.dogs)
                } else {
                    // Domain mapped to empty set → use default (all Dogs)
                    None
                }
            } else {
                // No domain, no dogs → use all Dogs
                None
            }
        }
    };

    // Inject domain-specific stimulus context if domain provided and no explicit context
    let enriched_context = if req.context.is_none() && req.domain.is_some() {
        let domain = Domain::parse(req.domain.as_deref());
        Some(domain.stimulus_context())
    } else {
        req.context
    };

    let deps = crate::pipeline::PipelineDeps {
        judge: &judge,
        storage: state.storage.as_ref(),
        embedding: state.embedding.as_ref(),
        usage: &state.usage,
        verdict_cache: &state.verdict_cache,
        metrics: &state.metrics,
        event_tx: Some(&state.event_tx),
        request_id: Some(uuid::Uuid::new_v4().to_string()),
        on_dog: None,
        expected_dog_count: judge.dog_ids().len(),
        enricher: state.enricher.as_deref(),
        domain_curations: state.domain_curations.as_ref(),
    };
    let result = crate::pipeline::run(
        req.content,
        enriched_context,
        req.domain,
        dogs.as_deref(),
        req.crystals,
        &deps,
    )
    .await
    .map_err(|e| {
        let status = match &e {
            crate::judge::JudgeError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %e, "judge pipeline failed");
        }
        (
            status,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    match result {
        crate::pipeline::PipelineResult::CacheHit {
            verdict,
            similarity,
        } => {
            tracing::info!(similarity = %format!("{:.4}", similarity), "verdict cache HIT");
            Ok(Json(verdict_response_cached(&verdict, similarity)))
        }
        crate::pipeline::PipelineResult::Evaluated {
            verdict,
            token_data,
            enriched_content,
        } => {
            let mut resp = verdict_to_response(verdict.as_ref());
            resp.token_data = token_data.map(|b| *b);
            resp.stimulus_content = enriched_content;
            Ok(Json(resp))
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
            Json(ErrorResponse {
                error: format!("Verdict {id} not found"),
            }),
        )),
        Err(e) => {
            tracing::warn!(verdict_id = %id, error = %e, "verdict get failed");
            Err(storage_error())
        }
    }
}

pub async fn list_verdicts_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<JudgeResponse>>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage.list_verdicts(20).await {
        Ok(verdicts) => Ok(Json(verdicts.iter().map(verdict_to_response).collect())),
        Err(e) => {
            tracing::warn!(error = %e, "verdicts list failed");
            Err(storage_error())
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
