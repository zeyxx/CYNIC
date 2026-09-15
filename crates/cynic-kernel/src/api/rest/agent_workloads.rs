use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use cynic_workload::{
    AgentWorkloadCompletionPublication,
    AgentWorkloadDelivery,
    AgentWorkloadDeliveryAcceptance,
    AgentWorkloadDeliveryStatus,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::types::{AppState, AuthenticatedPrincipal, ErrorResponse};

#[derive(Debug, Deserialize)]
pub struct AcceptRequest {
    pub consumer_instance_id: String,
    pub record_digest: String,
    pub lease_ttl_ms: u64,
    #[serde(default)]
    pub prior_lease_id: Option<String>,
    #[serde(default)]
    pub prior_fencing_token: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct DeliveryResponse<T> {
    schema: &'static str,
    status: &'static str,
    value: T,
}

pub async fn submit_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Json(delivery): Json<AgentWorkloadDelivery>,
) -> Result<Json<DeliveryResponse<AgentWorkloadDelivery>>, (StatusCode, Json<ErrorResponse>)> {
    principal
        .require_agent(&delivery.submitted_by)
        .map_err(|error| failure(StatusCode::FORBIDDEN, error))?;
    if delivery.status != AgentWorkloadDeliveryStatus::Pending {
        return Err(failure(
            StatusCode::UNPROCESSABLE_ENTITY,
            "new workload delivery must be pending",
        ));
    }
    let persisted = state
        .storage
        .submit_agent_workload_delivery(&delivery)
        .await
        .map_err(storage_failure)?;
    Ok(Json(DeliveryResponse {
        schema: "cynic.agent_workload_delivery_response.v1",
        status: "persisted",
        value: persisted,
    }))
}

pub async fn read_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Path(workload_id): Path<String>,
) -> Result<Json<DeliveryResponse<AgentWorkloadDelivery>>, (StatusCode, Json<ErrorResponse>)> {
    let delivery = state
        .storage
        .agent_workload_delivery(&workload_id)
        .await
        .map_err(storage_failure)?
        .ok_or_else(|| failure(StatusCode::NOT_FOUND, "workload delivery not found"))?;
    require_submitter_or_node(&principal, &delivery)?;
    Ok(Json(DeliveryResponse {
        schema: "cynic.agent_workload_delivery_response.v1",
        status: "observed",
        value: delivery,
    }))
}

pub async fn pending_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Path(node_id): Path<String>,
) -> Result<Json<DeliveryResponse<Vec<AgentWorkloadDelivery>>>, (StatusCode, Json<ErrorResponse>)> {
    principal
        .require_agent(&node_id)
        .map_err(|error| failure(StatusCode::FORBIDDEN, error))?;
    let deliveries = state
        .storage
        .pending_agent_workload_deliveries(&node_id)
        .await
        .map_err(storage_failure)?;
    Ok(Json(DeliveryResponse {
        schema: "cynic.agent_workload_delivery_list_response.v1",
        status: "observed",
        value: deliveries,
    }))
}

pub async fn accept_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Path((node_id, workload_id)): Path<(String, String)>,
    Json(request): Json<AcceptRequest>,
) -> Result<Json<DeliveryResponse<AgentWorkloadDelivery>>, (StatusCode, Json<ErrorResponse>)> {
    principal
        .require_agent(&node_id)
        .map_err(|error| failure(StatusCode::FORBIDDEN, error))?;
    let acceptance = AgentWorkloadDeliveryAcceptance {
        consumer_instance_id: request.consumer_instance_id,
        record_digest: request.record_digest,
        requested_at_ms: now_unix_ms(),
        lease_ttl_ms: request.lease_ttl_ms,
        prior_lease_id: request.prior_lease_id,
        prior_fencing_token: request.prior_fencing_token,
    };
    let delivery = state
        .storage
        .accept_agent_workload_delivery(&workload_id, &node_id, &acceptance)
        .await
        .map_err(storage_failure)?;
    Ok(Json(DeliveryResponse {
        schema: "cynic.agent_workload_delivery_response.v1",
        status: "accepted",
        value: delivery,
    }))
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

pub async fn completion_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Path((node_id, workload_id)): Path<(String, String)>,
    Json(publication): Json<AgentWorkloadCompletionPublication>,
) -> Result<Json<DeliveryResponse<AgentWorkloadDelivery>>, (StatusCode, Json<ErrorResponse>)> {
    principal
        .require_agent(&node_id)
        .map_err(|error| failure(StatusCode::FORBIDDEN, error))?;
    if publication.node_id != node_id || publication.workload_id != workload_id {
        return Err(failure(
            StatusCode::UNPROCESSABLE_ENTITY,
            "workload completion publication path binding mismatch",
        ));
    }
    let delivery = state
        .storage
        .complete_agent_workload_delivery(&publication)
        .await
        .map_err(storage_failure)?;
    Ok(Json(DeliveryResponse {
        schema: "cynic.agent_workload_delivery_response.v1",
        status: "terminal",
        value: delivery,
    }))
}

fn require_submitter_or_node(
    principal: &AuthenticatedPrincipal,
    delivery: &AgentWorkloadDelivery,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if principal.require_agent(&delivery.submitted_by).is_ok()
        || principal.require_agent(&delivery.target_node_id).is_ok()
    {
        Ok(())
    } else {
        Err(failure(
            StatusCode::FORBIDDEN,
            "workload delivery read-back requires its submitter or target node identity",
        ))
    }
}

fn storage_failure(
    error: crate::domain::storage::StorageError,
) -> (StatusCode, Json<ErrorResponse>) {
    let status = match &error {
        crate::domain::storage::StorageError::NotFound(_) => StatusCode::NOT_FOUND,
        crate::domain::storage::StorageError::InvalidInput(_) => StatusCode::UNPROCESSABLE_ENTITY,
        crate::domain::storage::StorageError::ConnectionFailed(_)
        | crate::domain::storage::StorageError::QueryFailed(_) => StatusCode::SERVICE_UNAVAILABLE,
    };
    failure(status, &error.to_string())
}

fn failure(status: StatusCode, error: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
}
