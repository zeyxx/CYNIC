use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use cynic_protocol::NodeControlPlan;
use serde::Serialize;

use super::types::{AppState, AuthenticatedPrincipal, ErrorResponse};

#[derive(Debug, Serialize)]
struct NodeControlResponse<T> {
    schema: &'static str,
    status: &'static str,
    value: T,
}

pub async fn desired_state_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Path(node_id): Path<String>,
) -> Response {
    if let Err(error) = principal.require_agent(&node_id) {
        return forbidden(error);
    }
    let Some(runtime) = state.node_control.as_ref() else {
        return unavailable();
    };
    match runtime.store.desired_state(&node_id).await {
        Ok(Some(value)) => Json(NodeControlResponse {
            schema: "cynic.node_desired_state_response.v1",
            status: "observed",
            value,
        })
        .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "node desired state is not declared".to_string(),
            }),
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

pub async fn publish_status_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Path(node_id): Path<String>,
    Json(status): Json<NodeControlPlan>,
) -> Response {
    if let Err(error) = principal.require_agent(&node_id) {
        return forbidden(error);
    }
    if status.node_id != node_id {
        return forbidden("status identity does not match the target node");
    }
    let Some(runtime) = state.node_control.as_ref() else {
        return unavailable();
    };
    match runtime.store.put_status(status.clone()).await {
        Ok(()) => Json(NodeControlResponse {
            schema: "cynic.node_status_publish_response.v1",
            status: "persisted",
            value: status,
        })
        .into_response(),
        Err(crate::domain::node_control::NodeControlStoreError::Invalid(error)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse { error }),
        )
            .into_response(),
        Err(crate::domain::node_control::NodeControlStoreError::Conflict(error)) => {
            (StatusCode::CONFLICT, Json(ErrorResponse { error })).into_response()
        }
        Err(crate::domain::node_control::NodeControlStoreError::Storage(_)) => unavailable(),
    }
}

pub async fn status_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Path(node_id): Path<String>,
) -> Response {
    if let Err(error) = principal.require_agent(&node_id) {
        return forbidden(error);
    }
    let Some(runtime) = state.node_control.as_ref() else {
        return unavailable();
    };
    match runtime.store.latest_status(&node_id).await {
        Ok(Some(value)) => Json(NodeControlResponse {
            schema: "cynic.node_status_response.v1",
            status: "observed",
            value,
        })
        .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "node status has not been published".to_string(),
            }),
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

fn forbidden(error: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
        .into_response()
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "node control read-back is unavailable".to_string(),
        }),
    )
        .into_response()
}
