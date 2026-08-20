use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use cynic_protocol::{
    NodeControlPlan, RuntimeReleaseAdmission, RuntimeReleaseDeclaration,
    SignedNodeControlPlan, admit_runtime_release,
};
use serde::Serialize;

use super::types::{AppState, AuthenticatedPrincipal, ErrorResponse};

#[derive(Debug, Serialize)]
struct NodeControlResponse<T> {
    schema: &'static str,
    status: &'static str,
    value: T,
}

#[derive(Debug, Serialize)]
struct RuntimeReleaseResponse {
    schema: &'static str,
    status: &'static str,
    admission: RuntimeReleaseAdmission,
    value: cynic_protocol::NodeDesiredState,
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

pub async fn signed_desired_state_handler(
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
    let desired = match runtime.store.desired_state(&node_id).await {
        Ok(Some(value)) => value,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "node desired state is not declared".to_string(),
                }),
            )
                .into_response();
        }
        Err(_) => return unavailable(),
    };

    let Some(enrollment) = state.node_enrollment.as_ref() else {
        return unavailable();
    };

    let now_unix_s = chrono::Utc::now().timestamp().max(0) as u64;
    let expires_at_unix_s = now_unix_s.saturating_add(3600);
    let mut plan = SignedNodeControlPlan::new(
        desired,
        enrollment.signer.key_id(),
        now_unix_s,
        expires_at_unix_s,
        String::new(),
    );

    let signing_bytes = plan.signing_bytes();
    let signature_hex = match enrollment.signer.sign(&signing_bytes) {
        Ok(sig) => sig,
        Err(_) => return unavailable(),
    };
    plan.signature_hex = signature_hex;

    Json(NodeControlResponse {
        schema: "cynic.signed_node_desired_state_response.v1",
        status: "signed",
        value: plan,
    })
    .into_response()
}

pub async fn declare_release_handler(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedPrincipal>,
    Path(node_id): Path<String>,
    Json(declaration): Json<RuntimeReleaseDeclaration>,
) -> Response {
    if let Err(error) = principal.require_agent(&declaration.requested_by) {
        return forbidden(error);
    }
    if declaration.desired_state.node_id != node_id {
        return invalid("runtime release target does not match the desired-state path");
    }
    let delivery = match state
        .storage
        .agent_workload_delivery(&declaration.workload_id)
        .await
    {
        Ok(Some(delivery)) => delivery,
        Ok(None) => return not_found("terminal build workload delivery is not retained"),
        Err(_) => return unavailable(),
    };
    let admission = admit_runtime_release(&declaration, &delivery);
    if !admission.admitted {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(admission)).into_response();
    }
    let desired = declaration.desired_state;
    let Some(runtime) = state.node_control.as_ref() else {
        return unavailable();
    };
    match runtime.store.put_desired_state(desired.clone()).await {
        Ok(()) => {}
        Err(crate::domain::node_control::NodeControlStoreError::Invalid(error)) => {
            return invalid(&error);
        }
        Err(crate::domain::node_control::NodeControlStoreError::Conflict(error)) => {
            return (StatusCode::CONFLICT, Json(ErrorResponse { error })).into_response();
        }
        Err(crate::domain::node_control::NodeControlStoreError::Storage(_)) => {
            return unavailable();
        }
    }
    match runtime.store.desired_state(&node_id).await {
        Ok(Some(read_back)) if read_back == desired => Json(RuntimeReleaseResponse {
            schema: "cynic.runtime_release_response.v1",
            status: "admitted_persisted_and_read_back",
            admission,
            value: read_back,
        })
        .into_response(),
        Ok(_) => unavailable(),
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

fn invalid(error: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
        .into_response()
}

fn not_found(error: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
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
