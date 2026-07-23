use super::response::coordination_error;
use super::types::{AppState, AuthenticatedPrincipal, ErrorResponse};
use crate::domain::coord::{LeaseGuard, LeaseMutationResult};
use crate::domain::lease_proof::ValidatedLeaseProof;
use axum::{Extension, extract::State, http::StatusCode, response::Json};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct LeaseGuardRequest {
    pub agent_id: String,
    pub target: String,
    pub lease_id: String,
    pub fencing_token: u64,
}
#[derive(Debug, Deserialize)]
pub struct FinalizeLeaseRequest {
    pub agent_id: String,
    pub target: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub proof_bundle: cynic_protocol::RuntimeProofBundle,
}

pub async fn coord_renew_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(req): Json<LeaseGuardRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let guard = validated_guard(&principal, req)?;
    let result = state.coord.renew_lease(&guard).await.map_err(|error| {
        tracing::warn!(error = ?error, "renew_lease failed");
        coordination_error()
    })?;
    lease_response(&state, "cynic_coord_renew", &guard, result).await
}
pub async fn coord_release_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(req): Json<LeaseGuardRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let guard = validated_guard(&principal, req)?;
    let result = state.coord.release(&guard).await.map_err(|error| {
        tracing::warn!(error = ?error, "release failed");
        coordination_error()
    })?;
    lease_response(&state, "cynic_coord_release", &guard, result).await
}
pub async fn coord_finalize_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(req): Json<FinalizeLeaseRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let proof_bundle = req.proof_bundle;
    let guard = validated_guard(
        &principal,
        LeaseGuardRequest {
            agent_id: req.agent_id,
            target: req.target,
            lease_id: req.lease_id,
            fencing_token: req.fencing_token,
        },
    )?;
    let now_unix_ms = chrono::Utc::now().timestamp_millis().max(0) as u128;
    let proof = ValidatedLeaseProof::validate(&proof_bundle, &guard.target, now_unix_ms)
        .map_err(|message| error(StatusCode::UNPROCESSABLE_ENTITY, &message))?;
    let result = state
        .coord
        .finalize_lease(&guard, &proof)
        .await
        .map_err(|error| {
            tracing::warn!(error = ?error, "finalize_lease failed");
            coordination_error()
        })?;
    lease_response(&state, "cynic_coord_finalize", &guard, result).await
}
fn validated_guard(
    principal: &AuthenticatedPrincipal,
    req: LeaseGuardRequest,
) -> Result<LeaseGuard, (StatusCode, Json<ErrorResponse>)> {
    crate::domain::coord::validate_agent_id(&req.agent_id)
        .map_err(|m| error(StatusCode::BAD_REQUEST, m))?;
    principal
        .require_agent(&req.agent_id)
        .map_err(|message| error(StatusCode::FORBIDDEN, message))?;
    if req.target.is_empty()
        || req.target.len() > 256
        || req.lease_id.is_empty()
        || req.lease_id.len() > 256
    {
        return bad_request("invalid target or lease_id");
    }
    if req.fencing_token == 0 {
        return bad_request("fencing_token must be greater than zero");
    }
    Ok(LeaseGuard {
        agent_id: req.agent_id,
        target: req.target,
        lease_id: req.lease_id,
        fencing_token: req.fencing_token,
    })
}
async fn lease_response(
    state: &Arc<AppState>,
    tool: &str,
    guard: &LeaseGuard,
    result: LeaseMutationResult,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if !result.is_applied() {
        return Err(error(
            StatusCode::CONFLICT,
            "lease expired, superseded, or guard did not match",
        ));
    }
    let _ = state
        .coord
        .store_audit(
            tool,
            &guard.agent_id,
            &serde_json::to_string(&result).unwrap_or_else(|_| "{}".into()),
        )
        .await;
    Ok(Json(
        serde_json::json!({"status": "applied", "result": result}),
    ))
}
fn bad_request<T>(message: &str) -> Result<T, (StatusCode, Json<ErrorResponse>)> {
    Err(error(StatusCode::BAD_REQUEST, message))
}
fn error(status: StatusCode, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}
