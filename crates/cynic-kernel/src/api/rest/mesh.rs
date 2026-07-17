//! REST handlers for mesh network coordination (WireGuard V3).

use axum::{extract::State, http::StatusCode, response::Json};
use cynic_node::mesh_registry::validate_mesh_key_publication;
use std::sync::Arc;

use super::types::{AppState, ErrorResponse, PublishMeshKeyRequest, PublishMeshKeyResponse};

/// POST /mesh/publish
/// POST /mesh/publish
/// Publishes a node's public key for its statically assigned mesh IP.
pub async fn mesh_publish_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PublishMeshKeyRequest>,
) -> Result<Json<PublishMeshKeyResponse>, (StatusCode, Json<ErrorResponse>)> {
    if let Err(error) = validate_mesh_key_publication(&req.mesh_ip, &req.public_key) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: error.message().into(),
            }),
        ));
    }

    // Call store layer to publish mesh key
    let node = state
        .storage
        .publish_mesh_key(&req.mesh_ip, &req.public_key)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to publish mesh key in storage");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to publish mesh key: {e}"),
                }),
            )
        })?;

    Ok(Json(PublishMeshKeyResponse {
        status: "published".to_string(),
        mesh_ip: node.mesh_ip,
        public_key: node.public_key,
    }))
}

/// GET /mesh/nodes
/// Lists all registered mesh nodes.
pub async fn list_mesh_nodes_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<crate::domain::storage::MeshNode>>, (StatusCode, Json<ErrorResponse>)> {
    let nodes = state.storage.list_mesh_nodes().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list mesh nodes: {e}"),
            }),
        )
    })?;
    Ok(Json(nodes))
}

/// GET /mesh/routes
/// Lists all registered mesh routes.
pub async fn list_mesh_routes_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<crate::domain::storage::MeshRoute>>, (StatusCode, Json<ErrorResponse>)> {
    let routes = state.storage.list_mesh_routes().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list mesh routes: {e}"),
            }),
        )
    })?;
    Ok(Json(routes))
}
