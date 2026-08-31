//! REST handlers for coordination registration, claims, heartbeats, and observation.
//! These expose the same CoordPort methods as MCP, enabling hooks and external
//! clients to coordinate without MCP stdio.

use axum::{Extension, extract::State, http::StatusCode, response::Json};
use cynic_host::runtime_registry::resolve_runtime_registry_path;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

use super::response::coordination_error;
use super::types::{AppState, AuthenticatedPrincipal, ErrorResponse};
use crate::domain::coord::ClaimResult;

const COORD_HANDLER_TIMEOUT: Duration = Duration::from_secs(5);

fn validate_agent_id(id: &str) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    crate::domain::coord::validate_agent_id(id).map_err(|msg| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: msg.into() }),
        )
    })
}

fn require_agent_principal(
    principal: &AuthenticatedPrincipal,
    agent_id: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    principal.require_agent(agent_id).map_err(|message| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: message.into(),
            }),
        )
    })
}

// ── Request types ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub agent_id: String,
    pub intent: String,
    pub agent_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentIdentityRequest {
    pub agent_id: String,
    pub agent_kind: Option<String>,
    pub credential_kind: Option<String>,
    pub credential_reference: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClaimRequest {
    pub agent_id: String,
    pub target: String,
    pub claim_type: Option<String>,
    pub ttl_seconds: Option<u64>,
    pub budget_actions: Option<u64>,
    pub budget_wall_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct BatchClaimRequest {
    pub agent_id: String,
    pub targets: Vec<String>,
    pub claim_type: Option<String>,
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CoordHeartbeatRequest {
    pub agent_id: String,
    pub lease_id: Option<String>,
    pub fencing_token: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ScopeRequest {
    pub agent_id: String,
    pub scope: String,
}

// ── Handlers ──────────────────────────────────────────────

pub async fn coord_identify_handler(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<AgentIdentityRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    validate_agent_id(&req.agent_id)?;

    if req.agent_kind.as_ref().is_some_and(|k| k.len() > 64) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "agent_kind must be under 64 characters".into(),
            }),
        ));
    }

    let agent_kind = req.agent_kind.unwrap_or_else(|| "external".into());
    let credential_kind = req
        .credential_kind
        .unwrap_or_else(|| "opaque_reference".into());
    let attested_at_unix_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;

    let identity = serde_json::json!({
        "schema": "cynic.agent_identity.v1",
        "agent_id": req.agent_id,
        "agent_kind": agent_kind,
        "credential_kind": credential_kind,
        "credential_reference": req.credential_reference,
        "attested_at_unix_ms": attested_at_unix_ms,
        "status": "identified",
    });

    Ok(Json(identity))
}

pub async fn coord_register_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    validate_agent_id(&req.agent_id)?;
    require_agent_principal(&principal, &req.agent_id)?;
    if req.intent.is_empty() || req.intent.chars().count() > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "intent must be 1-500 characters".into(),
            }),
        ));
    }
    let agent_type = req.agent_type.unwrap_or_else(|| "unknown".into());
    if agent_type.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "agent_type must be under 64 characters".into(),
            }),
        ));
    }

    state
        .coord
        .register_agent(&req.agent_id, &agent_type, &req.intent)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "coord register failed");
            coordination_error()
        })?;

    // Heartbeat: keep session alive (REST has no implicit heartbeat like MCP)
    let _ = state.coord.heartbeat(&req.agent_id).await; // ok: fire-and-forget

    let _ = state
        .coord
        .store_audit(
            // ok: fire-and-forget
            "cynic_coord_register",
            &req.agent_id,
            &serde_json::json!({"intent": req.intent, "agent_type": agent_type, "source": "rest"})
                .to_string(),
        )
        .await;

    let _ = state
        .event_tx
        .send(crate::domain::events::KernelEvent::SessionRegistered {
            agent_id: req.agent_id.clone(),
        }); // ok: no subscribers = silent

    Ok(Json(serde_json::json!({
        "status": "registered",
        "agent_id": req.agent_id,
    })))
}

pub async fn coord_claim_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(req): Json<ClaimRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    validate_agent_id(&req.agent_id)?;
    require_agent_principal(&principal, &req.agent_id)?;
    if req.target.is_empty() || req.target.len() > 256 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "target must be 1-256 characters".into(),
            }),
        ));
    }
    let claim_type = req.claim_type.unwrap_or_else(|| "file".into());
    if claim_type.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "claim_type must be under 64 characters".into(),
            }),
        ));
    }

    // Enforce TTL bounds (30s to 24h)
    let ttl_seconds = req.ttl_seconds.unwrap_or(1800).clamp(30, 86400);

    // K15 Gate: Enforce compliance threshold before allowing claims
    let observations = state
        .storage
        .list_observations_raw(None, Some(&req.agent_id), 500)
        .await
        .unwrap_or_default();

    let comp =
        crate::domain::compliance::score_session(&req.agent_id, &req.agent_id, &observations);
    if comp.score < crate::domain::dog::PHI_INV3 {
        tracing::error!(agent_id = %req.agent_id, score = %comp.score, "compliance: claim rejected");
        return Err((
            StatusCode::PRECONDITION_FAILED,
            Json(ErrorResponse {
                error: format!(
                    "CRITICAL NON-COMPLIANCE (score={:.3}). Claim REJECTED. Warnings: {}",
                    comp.score,
                    comp.warnings.join("; ")
                ),
            }),
        ));
    }

    match state
        .coord
        .claim(&req.agent_id, &req.target, &claim_type)
        .await
    {
        Ok(ClaimResult::Claimed(grant)) => {
            let _ = state.coord.heartbeat(&req.agent_id).await;
            let budget_actions = req.budget_actions.unwrap_or(50);
            let budget_wall_seconds = req.budget_wall_seconds.unwrap_or(3600);
            let _ = state
                .coord
                .store_audit(
                    "cynic_coord_claim",
                    &req.agent_id,
                    &serde_json::json!({
                        "target": req.target,
                        "claim_type": claim_type,
                        "ttl_seconds": ttl_seconds,
                        "budget_actions": budget_actions,
                        "budget_wall_seconds": budget_wall_seconds,
                        "grant": &grant,
                        "source": "rest"
                    })
                    .to_string(),
                )
                .await;
            Ok(Json(serde_json::json!({
                "status": "claimed",
                "agent_id": req.agent_id,
                "target": req.target,
                "ttl_seconds": ttl_seconds,
                "budget_actions": budget_actions,
                "budget_wall_seconds": budget_wall_seconds,
                "lease": grant,
            })))
        }
        Ok(ClaimResult::Conflict(conflicts)) => {
            let conflict_list: Vec<_> = conflicts
                .iter()
                .map(|c| serde_json::json!({"agent_id": &c.agent_id, "claimed_at": &c.claimed_at}))
                .collect();
            let _ = state.coord.store_audit(
                "cynic_coord_claim", &req.agent_id,
                &serde_json::json!({"target": req.target, "claim_type": claim_type, "conflicts": &conflict_list, "source": "rest"}).to_string(),
            ).await;
            Ok(Json(serde_json::json!({
                "status": "conflict",
                "agent_id": req.agent_id,
                "target": req.target,
                "conflicts": conflict_list,
            })))
        }
        Err(e) => {
            tracing::warn!(error = %e, "coord claim failed");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "coordination unavailable".into(),
                }),
            ))
        }
    }
}

pub async fn coord_claim_batch_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(req): Json<BatchClaimRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    validate_agent_id(&req.agent_id)?;
    require_agent_principal(&principal, &req.agent_id)?;
    if req.targets.is_empty() || req.targets.len() > 20 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "targets must be 1-20 items".into(),
            }),
        ));
    }
    for t in &req.targets {
        if t.is_empty() || t.len() > 256 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!(
                        "target '{}' must be 1-256 characters",
                        t.chars().take(32).collect::<String>()
                    ),
                }),
            ));
        }
    }

    let claim_type = req.claim_type.unwrap_or_else(|| "file".into());
    if claim_type.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "claim_type must be under 64 characters".into(),
            }),
        ));
    }

    let result = state
        .coord
        .claim_batch(&req.agent_id, &req.targets, &claim_type)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "coord claim-batch failed");
            coordination_error()
        })?;

    let _ = state.coord.store_audit( // ok: fire-and-forget
        "cynic_coord_claim_batch", &req.agent_id,
        &serde_json::json!({"targets": req.targets, "claimed": result.claimed_count(), "conflicts": result.conflict_count(), "source": "rest"}).to_string(),
    ).await;

    Ok(Json(serde_json::json!({
        "agent_id": req.agent_id,
        "result": result,
    })))
}

pub async fn coord_heartbeat_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(req): Json<CoordHeartbeatRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    validate_agent_id(&req.agent_id)?;
    require_agent_principal(&principal, &req.agent_id)?;

    state.coord.heartbeat(&req.agent_id).await.map_err(|e| {
        tracing::warn!(error = %e, "coord heartbeat failed");
        coordination_error()
    })?;

    Ok(Json(serde_json::json!({
        "status": "heartbeat_accepted",
        "agent_id": req.agent_id,
    })))
}

pub async fn coord_scope_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(req): Json<ScopeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    validate_agent_id(&req.agent_id)?;
    require_agent_principal(&principal, &req.agent_id)?;
    if req.scope.is_empty() || req.scope.chars().count() > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "scope must be 1-500 characters".into(),
            }),
        ));
    }

    state
        .coord
        .scope_update(&req.agent_id, &req.scope)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "coord scope update failed");
            coordination_error()
        })?;

    Ok(Json(serde_json::json!({
        "status": "scope_updated",
        "agent_id": req.agent_id,
    })))
}

/// GET /coord/who — show all active agents and their claims.
/// Optional query param: ?agent_id=xxx to filter by agent.
pub async fn coord_who_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::BTreeMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let agent_filter = params.get("agent_id").map(|s| s.as_str());

    let snapshot = tokio::time::timeout(COORD_HANDLER_TIMEOUT, state.coord.who(agent_filter))
        .await
        .map_err(|_e| {
            tracing::warn!("coord who timed out");
            (
                StatusCode::GATEWAY_TIMEOUT,
                Json(ErrorResponse {
                    error: "coordination query timed out".into(),
                }),
            )
        })?
        .map_err(|e| {
            tracing::warn!(error = %e, "coord who failed");
            coordination_error()
        })?;

    Ok(Json(serde_json::json!({
        "agents": snapshot.agents,
        "claims": snapshot.claims,
    })))
}

/// GET /coord/topology — exposes the current fleet topology from infra/registry.json.
pub async fn coord_topology_handler()
-> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let registry_path = resolve_runtime_registry_path(None).map_err(|error| {
        tracing::warn!(error = %error, "failed to resolve runtime registry");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "topology unavailable".into(),
            }),
        )
    })?;
    let content = std::fs::read_to_string(&registry_path).map_err(|e| {
        tracing::warn!(error = %e, path = %registry_path.display(), "failed to read registry.json");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "topology unavailable".into(),
            }),
        )
    })?;

    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        tracing::warn!(error = %e, "failed to parse registry.json");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "topology malformed".into(),
            }),
        )
    })?;

    Ok(Json(json))
}
