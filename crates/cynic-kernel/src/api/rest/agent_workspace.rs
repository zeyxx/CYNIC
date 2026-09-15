use std::sync::Arc;

use axum::{Extension, extract::State, http::StatusCode, response::Json};
use cynic_workload::{
    AgentWorkspaceClaimAdmission,
    AgentWorkspaceClaimIntent,
    AgentWorkspaceClaimObservation,
    AgentWorkspaceClaimOutcome,
};
use cynic_protocol::{
    LeaseGuard,
};
use serde::Deserialize;

use super::types::{AppState, AuthenticatedPrincipal, ErrorResponse};

#[derive(Debug, Deserialize)]
pub struct ReserveRequest {
    pub intent: AgentWorkspaceClaimIntent,
    pub admission: AgentWorkspaceClaimAdmission,
    pub now_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct ListRequest {
    pub agent_id: String,
    pub repository_id: String,
    pub now_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct RenewRequest {
    pub guard: LeaseGuard,
    pub requested_ttl_ms: u64,
    pub now_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseRequest {
    pub guard: LeaseGuard,
    pub now_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct FinalizeRequest {
    pub guard: LeaseGuard,
    pub proof_ref: String,
    pub now_ms: u64,
}

pub async fn reserve_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(request): Json<ReserveRequest>,
) -> Result<Json<AgentWorkspaceClaimOutcome>, (StatusCode, Json<ErrorResponse>)> {
    require_agent(&principal, &request.intent.agent_id)?;
    state
        .storage
        .reserve_agent_workspace_claim(&request.intent, &request.admission, request.now_ms)
        .await
        .map(Json)
        .map_err(workspace_error)
}

pub async fn list_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(request): Json<ListRequest>,
) -> Result<Json<Vec<AgentWorkspaceClaimObservation>>, (StatusCode, Json<ErrorResponse>)> {
    require_agent(&principal, &request.agent_id)?;
    state
        .storage
        .list_agent_workspace_claims(&request.repository_id, request.now_ms)
        .await
        .map(Json)
        .map_err(workspace_error)
}

pub async fn renew_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(request): Json<RenewRequest>,
) -> Result<Json<AgentWorkspaceClaimObservation>, (StatusCode, Json<ErrorResponse>)> {
    require_agent(&principal, &request.guard.agent_id)?;
    state
        .storage
        .renew_agent_workspace_claim(&request.guard, request.requested_ttl_ms, request.now_ms)
        .await
        .map(Json)
        .map_err(workspace_error)
}

pub async fn release_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(request): Json<ReleaseRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    require_agent(&principal, &request.guard.agent_id)?;
    state
        .storage
        .release_agent_workspace_claim(&request.guard, request.now_ms)
        .await
        .map_err(workspace_error)?;
    Ok(Json(serde_json::json!({"status": "released"})))
}

pub async fn finalize_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(request): Json<FinalizeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    require_agent(&principal, &request.guard.agent_id)?;
    state
        .storage
        .finalize_agent_workspace_claim(&request.guard, &request.proof_ref, request.now_ms)
        .await
        .map_err(workspace_error)?;
    Ok(Json(serde_json::json!({"status": "finalized"})))
}

fn require_agent(
    principal: &AuthenticatedPrincipal,
    agent_id: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    crate::domain::coord::validate_agent_id(agent_id)
        .map_err(|message| error(StatusCode::BAD_REQUEST, message))?;
    principal
        .require_agent(agent_id)
        .map_err(|message| error(StatusCode::FORBIDDEN, message))
}

fn workspace_error(
    failure: cynic_node::ports::agent_workspace_port::AgentWorkspacePortError,
) -> (StatusCode, Json<ErrorResponse>) {
    let status = if failure.code.contains("rejected") || failure.code.contains("conflict") {
        StatusCode::CONFLICT
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    error(status, &format!("{}:{}", failure.code, failure.message))
}

fn error(status: StatusCode, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: message.to_string(),
        }),
    )
}
