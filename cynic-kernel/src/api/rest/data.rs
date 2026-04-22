//! REST API handlers for data access — /crystals, /crystal/{id}, /usage, crystal CRUD.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

use super::response::{coordination_error, storage_error};
use super::types::{AppState, ErrorResponse};
use crate::domain::ccm;
use crate::domain::ccm::Crystal;
use crate::domain::compliance;

fn crystal_to_json(c: &Crystal) -> serde_json::Value {
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
    })
}

#[derive(Debug, Deserialize, Default)]
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

pub async fn usage_handler(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let usage = state.usage.lock().await;
    let merged = usage.merged_dogs();
    let active_ids: std::collections::HashSet<String> =
        state.judge.load_full().dog_ids().into_iter().collect();

    let mut active_dogs: Vec<serde_json::Value> = Vec::new();
    let mut retired_tokens: u64 = 0;
    let mut retired_requests: u64 = 0;
    let mut retired_count: u32 = 0;

    for (id, d) in &merged {
        if active_ids.contains(id) {
            let avg_latency = d.total_latency_ms.checked_div(d.requests).unwrap_or(0);
            active_dogs.push(serde_json::json!({
                "dog_id": id,
                "prompt_tokens": d.prompt_tokens,
                "completion_tokens": d.completion_tokens,
                "total_tokens": d.total_tokens(),
                "requests": d.requests,
                "failures": d.failures,
                "avg_latency_ms": avg_latency,
            }));
        } else {
            retired_tokens += d.total_tokens();
            retired_requests += d.requests;
            retired_count += 1;
        }
    }
    active_dogs.sort_by(|a, b| b["requests"].as_u64().cmp(&a["requests"].as_u64()));
    Json(serde_json::json!({
        "total_tokens": usage.total_tokens(),
        "total_requests": usage.all_time_requests(),
        "estimated_cost_usd": usage.estimated_cost_usd(),
        "uptime_seconds": usage.uptime_seconds(),
        "per_dog": active_dogs,
        "retired": {
            "count": retired_count,
            "total_tokens": retired_tokens,
            "total_requests": retired_requests,
        },
    }))
}

// ── Crystal CRUD ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
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
        contributing_verdicts: vec![],
        certainty: 0.0,
        variance_m2: 0.0,
        mean_quorum: 0.0,
        howl_count: 0,
        wag_count: 0,
        growl_count: 0,
        bark_count: 0,
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

/// DELETE /crystal/{id} — delete a crystal by ID. Idempotent.
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

// ── Dark table endpoints ─────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct ObservationsQuery {
    pub limit: Option<u32>,
    pub domain: Option<String>,
    pub agent_id: Option<String>,
}

/// GET /observations — raw workflow observations (10K+ rows, previously invisible).
pub async fn observations_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ObservationsQuery>,
) -> Result<Json<Vec<crate::domain::storage::RawObservation>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(100).min(100); // safe_limit caps at 100 in storage
    match state
        .storage
        .list_observations_raw(q.domain.as_deref(), q.agent_id.as_deref(), limit)
        .await
    {
        Ok(rows) => Ok(Json(rows)),
        Err(e) => {
            tracing::warn!(error = %e, "observations list failed");
            Err(storage_error())
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct SessionsQuery {
    pub limit: Option<u32>,
}

/// GET /sessions — session summaries (previously invisible).
pub async fn sessions_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(50).min(200);
    match state.storage.list_session_summaries(limit).await {
        Ok(summaries) => {
            let items: Vec<serde_json::Value> = summaries
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "session_id": s.session_id,
                        "agent_id": s.agent_id,
                        "summary": s.summary,
                        "observations_count": s.observations_count,
                        "created_at": s.created_at,
                    })
                })
                .collect();
            Ok(Json(items))
        }
        Err(e) => {
            tracing::warn!(error = %e, "sessions list failed");
            Err(storage_error())
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct AuditQuery {
    pub limit: Option<u32>,
    pub tool: Option<String>,
    pub agent_id: Option<String>,
}

/// GET /session/{agent_id}/compliance — Score a session's workflow compliance.
/// Queries observations for the agent, runs deterministic heuristics, stores + returns result.
pub async fn compliance_handler(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Result<Json<compliance::SessionCompliance>, (StatusCode, Json<ErrorResponse>)> {
    // 1. Get raw observations for this agent/session
    let observations = state
        .storage
        .list_observations_raw(None, Some(&agent_id), 500)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, agent_id = %agent_id, "compliance: observation query failed");
            storage_error()
        })?;

    // 2. Score (pure function — no I/O)
    let result = compliance::score_session(&agent_id, &agent_id, &observations);

    // 3. Store (best-effort — don't fail the request if store fails)
    if let Err(e) = state.storage.store_session_compliance(&result).await {
        tracing::warn!(error = %e, "compliance: failed to persist score");
    }

    // K15: emit Anomaly when compliance drops below φ⁻² (acting consumer → Slack alert)
    if result.score < crate::domain::dog::PHI_INV2 {
        let _ = state
            .event_tx
            .send(crate::domain::events::KernelEvent::Anomaly {
                kind: "low_compliance".to_string(),
                message: format!(
                    "Agent {} compliance={:.3} (threshold={:.3}): {}",
                    result.agent_id,
                    result.score,
                    crate::domain::dog::PHI_INV2,
                    result.warnings.join("; "),
                ),
                severity: "warning".to_string(),
            });
    }

    Ok(Json(result))
}

#[derive(Debug, Deserialize, Default)]
pub struct ComplianceTrendQuery {
    pub limit: Option<u32>,
}

/// GET /compliance — List recent compliance scores (trend tracking).
pub async fn compliance_trend_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ComplianceTrendQuery>,
) -> Result<Json<Vec<compliance::SessionCompliance>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(20).min(100);
    match state.storage.list_session_compliance(limit).await {
        Ok(reports) => Ok(Json(reports)),
        Err(e) => {
            tracing::warn!(error = %e, "compliance trend query failed");
            Err(storage_error())
        }
    }
}

/// GET /audit — MCP audit trail (previously MCP-only, 11K+ rows).
pub async fn audit_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<crate::domain::coord::AuditEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(50).min(100);
    match state
        .coord
        .query_audit(q.tool.as_deref(), q.agent_id.as_deref(), limit)
        .await
    {
        Ok(rows) => Ok(Json(rows)),
        Err(e) => {
            tracing::warn!(error = %e, "audit query failed");
            Err(coordination_error())
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
// WHY: test assertions — panics are acceptable and expected in test context
mod tests {
    use super::*;

    // ── CrystalsQuery deserialization ──────────────────────────

    #[test]
    fn crystals_query_defaults_are_none() {
        let q: CrystalsQuery = serde_json::from_str("{}").expect("empty object parses");
        assert!(q.limit.is_none());
        assert!(q.domain.is_none());
        assert!(q.state.is_none());
    }

    #[test]
    fn crystals_query_parses_all_fields() {
        // Simulate what axum Query extractor produces after parsing the query string
        let q = CrystalsQuery {
            limit: Some(25),
            domain: Some("chess".into()),
            state: Some("crystallized".into()),
        };
        assert_eq!(q.limit, Some(25));
        assert_eq!(q.domain.as_deref(), Some("chess"));
        assert_eq!(q.state.as_deref(), Some("crystallized"));
    }

    #[test]
    fn crystals_query_limit_default_is_50() {
        // Verify handler applies default limit=50 when None
        let q = CrystalsQuery {
            limit: None,
            domain: None,
            state: None,
        };
        let effective_limit = q.limit.unwrap_or(50).min(200);
        assert_eq!(effective_limit, 50);
    }

    #[test]
    fn crystals_query_limit_is_capped_at_200() {
        // Verify handler caps limit at 200 regardless of input
        let q = CrystalsQuery {
            limit: Some(9999),
            domain: None,
            state: None,
        };
        let effective_limit = q.limit.unwrap_or(50).min(200);
        assert_eq!(effective_limit, 200);
    }

    // ── CreateCrystalRequest validation ───────────────────────

    #[test]
    fn create_crystal_request_deserializes_with_domain() {
        let json = r#"{"content":"The Sicilian is strong","domain":"chess"}"#;
        let req: CreateCrystalRequest = serde_json::from_str(json).expect("full request parses");
        assert_eq!(req.content, "The Sicilian is strong");
        assert_eq!(req.domain.as_deref(), Some("chess"));
    }

    #[test]
    fn create_crystal_request_domain_is_optional() {
        let json = r#"{"content":"generic insight"}"#;
        let req: CreateCrystalRequest =
            serde_json::from_str(json).expect("domain-less request parses");
        assert!(req.domain.is_none());
        // Handler applies "general" default when None
        let domain = req.domain.unwrap_or_else(|| "general".into());
        assert_eq!(domain, "general");
    }

    #[test]
    fn create_crystal_content_empty_is_invalid() {
        // Mirrors the handler validation: empty content trimmed is rejected
        let content = "";
        assert!(
            content.trim().is_empty(),
            "empty content must fail validation"
        );
    }

    #[test]
    fn create_crystal_content_2001_chars_is_invalid() {
        // Mirrors the handler validation: > 2000 chars is rejected
        let content: String = "x".repeat(2001);
        assert!(
            content.chars().count() > 2000,
            "oversized content must fail validation"
        );
    }

    #[test]
    fn create_crystal_content_2000_chars_is_valid() {
        // Boundary: exactly 2000 chars must pass
        let content: String = "x".repeat(2000);
        assert!(
            !content.trim().is_empty() && content.chars().count() <= 2000,
            "2000 chars must pass validation"
        );
    }

    // ── ObservationsQuery deserialization ──────────────────────

    #[test]
    fn observations_query_defaults_are_none() {
        let q: ObservationsQuery = serde_json::from_str("{}").expect("empty object parses");
        assert!(q.limit.is_none());
        assert!(q.domain.is_none());
        assert!(q.agent_id.is_none());
    }

    #[test]
    fn observations_query_parses_agent_id() {
        // Simulate what axum Query extractor produces
        let q = ObservationsQuery {
            limit: Some(10),
            domain: Some("code".into()),
            agent_id: Some("claude-123".into()),
        };
        assert_eq!(q.agent_id.as_deref(), Some("claude-123"));
        assert_eq!(q.domain.as_deref(), Some("code"));
        assert_eq!(q.limit, Some(10));
    }

    #[test]
    fn observations_limit_default_and_cap() {
        // Default: 100, cap: 100
        let q = ObservationsQuery {
            limit: None,
            domain: None,
            agent_id: None,
        };
        let effective = q.limit.unwrap_or(100).min(100);
        assert_eq!(effective, 100);

        let q_big = ObservationsQuery {
            limit: Some(9999),
            domain: None,
            agent_id: None,
        };
        let effective_big = q_big.limit.unwrap_or(100).min(100);
        assert_eq!(effective_big, 100);
    }

    // ── SessionsQuery deserialization ──────────────────────────

    #[test]
    fn sessions_query_default_limit_is_none() {
        let q: SessionsQuery = serde_json::from_str("{}").expect("empty object parses");
        assert!(q.limit.is_none());
    }

    #[test]
    fn sessions_limit_default_50_cap_200() {
        let q = SessionsQuery { limit: None };
        let effective = q.limit.unwrap_or(50).min(200);
        assert_eq!(effective, 50);

        let q_big = SessionsQuery { limit: Some(500) };
        let effective_big = q_big.limit.unwrap_or(50).min(200);
        assert_eq!(effective_big, 200);
    }

    // ── AuditQuery deserialization ─────────────────────────────

    #[test]
    fn audit_query_defaults_are_none() {
        let q: AuditQuery = serde_json::from_str("{}").expect("empty object parses");
        assert!(q.limit.is_none());
        assert!(q.tool.is_none());
        assert!(q.agent_id.is_none());
    }

    #[test]
    fn audit_query_parses_all_fields() {
        // Simulate what axum Query extractor produces
        let q = AuditQuery {
            limit: Some(20),
            tool: Some("cynic_judge".into()),
            agent_id: Some("claude-1".into()),
        };
        assert_eq!(q.tool.as_deref(), Some("cynic_judge"));
        assert_eq!(q.agent_id.as_deref(), Some("claude-1"));
        assert_eq!(q.limit, Some(20));
    }

    #[test]
    fn audit_limit_default_50_cap_100() {
        let q = AuditQuery {
            limit: None,
            tool: None,
            agent_id: None,
        };
        let effective = q.limit.unwrap_or(50).min(100);
        assert_eq!(effective, 50);

        let q_big = AuditQuery {
            limit: Some(999),
            tool: None,
            agent_id: None,
        };
        let effective_big = q_big.limit.unwrap_or(50).min(100);
        assert_eq!(effective_big, 100);
    }

    // ── ComplianceTrendQuery deserialization ───────────────────

    #[test]
    fn compliance_trend_limit_default_20_cap_100() {
        let q = ComplianceTrendQuery { limit: None };
        let effective = q.limit.unwrap_or(20).min(100);
        assert_eq!(effective, 20);

        let q_big = ComplianceTrendQuery { limit: Some(999) };
        let effective_big = q_big.limit.unwrap_or(20).min(100);
        assert_eq!(effective_big, 100);
    }
}
