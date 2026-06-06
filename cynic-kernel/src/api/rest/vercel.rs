//! Vercel REST API handlers.

use crate::{
    api::rest::AppState, vercel::client::VercelClient, vercel::types::CreateDeploymentRequest,
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{error, info};

#[derive(Debug, Deserialize)]
pub struct DeployRequest {
    pub app_name: String,
}

pub async fn list_deployments_handler(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<DeployRequest>,
) -> impl IntoResponse {
    info!(app_name = %payload.app_name, "Vercel handler: listing deployments");

    let token = match std::env::var("VERCEL_API_TOKEN") {
        Ok(t) => {
            info!("VERCEL_API_TOKEN found");
            t
        }
        Err(_) => {
            error!("VERCEL_API_TOKEN not set");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "VERCEL_API_TOKEN not set",
            )
                .into_response();
        }
    };

    let client = VercelClient::new(token);
    match client.list_deployments(&payload.app_name).await {
        Ok(deployments) => {
            info!(
                count = deployments.len(),
                "Successfully fetched deployments from Vercel"
            );
            (StatusCode::OK, Json(deployments)).into_response()
        }
        Err(e) => {
            error!(error = ?e, "Failed to fetch deployments from Vercel");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(e)).into_response()
        }
    }
}

pub async fn create_deployment_handler(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<CreateDeploymentRequest>,
) -> impl IntoResponse {
    info!(name = %payload.name, "Vercel handler: creating deployment");

    let token = match std::env::var("VERCEL_API_TOKEN") {
        Ok(t) => t,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "VERCEL_API_TOKEN not set",
            )
                .into_response();
        }
    };

    let client = VercelClient::new(token);
    match client.create_deployment(payload).await {
        Ok(deployment) => (StatusCode::OK, Json(deployment)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(e)).into_response(),
    }
}
