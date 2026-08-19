//! HTTP server for the builder runtime.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use cynic_protocol::{
    AgentWorkloadRecord, AgentWorkloadState, BuildAgentRequest, BuildSourceReference,
    compose_build_agent_workload,
};

use crate::ports::agent_workload_store_port::AgentWorkloadStorePort;

#[derive(Clone)]
pub struct BuilderHttpConfig {
    pub listen_addr: SocketAddr,
    pub state_root: String,
    pub workspace_root: String,
    pub node_id: String,
}

#[derive(Clone)]
struct AppState {
    config: BuilderHttpConfig,
    store: Arc<dyn AgentWorkloadStorePort>,
}

#[derive(Debug, Deserialize)]
pub struct ForgejoPayload {
    #[serde(default)]
    pub before: String,
    #[serde(default)]
    pub after: String,
    #[serde(default)]
    pub reref: Option<ForgejoRef>,
    #[serde(default)]
    pub pusher: Option<ForgejoAuthor>,
    #[serde(default)]
    pub commits: Vec<ForgejoCommit>,
}

#[derive(Debug, Deserialize)]
pub struct ForgejoRef { pub id: String }

#[derive(Debug, Deserialize)]
pub struct ForgejoAuthor { pub name: String, pub email: String }

#[derive(Debug, Deserialize)]
pub struct ForgejoCommit { pub id: String, pub message: String, pub timestamp: Option<String> }

#[derive(Debug, Serialize)]
pub struct AcceptanceProof {
    pub schema: &'static str,
    pub workload_id: String,
    pub commit_sha: String,
    pub accepted_at_ms: i64,
    pub status: &'static str,
}

pub fn app(config: BuilderHttpConfig, store: Arc<dyn AgentWorkloadStorePort>) -> Router {
    let state = AppState { config, store };
    Router::new()
        .route("/health", get(health))
        .route("/webhook/forgejo", post(webhook_forgejo))
        .route("/proofs/:id", get(get_proof))
        .with_state(state)
}

async fn health() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

async fn get_proof(id: Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    let _ = id;
    Err(StatusCode::NOT_FOUND)
}

async fn webhook_forgejo(
    State(state): State<AppState>,
    Json(payload): Json<ForgejoPayload>,
) -> Result<Json<AcceptanceProof>, (StatusCode, String)> {
    let commit_sha = if !payload.after.is_empty() {
        payload.after.clone()
    } else if let Some(reref) = &payload.reref {
        reref.id.clone()
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            "missing commit sha (after or reref.id)".into(),
        ));
    };

    let workload_id = format!("forgejo-{}", Uuid::new_v4());
    let accepted_at_ms = Utc::now().timestamp_millis();

    let request = BuildAgentRequest {
        schema: "cynic.build_agent_request.v1".into(),
        request_id: workload_id.clone(),
        requested_by: payload.pusher.as_ref().map(|p| p.name.clone()).unwrap_or_else(|| "forgejo".into()),
        source_commit: commit_sha.clone(),
        source: BuildSourceReference {
            schema: "cynic.build.source_reference.v1".into(),
            repository_id: "cynic".into(),
            transport_uri: "https://forgejo.synapse.cynic".into(),
            revision: commit_sha.clone(),
            authority_ref: "forgejo".into(),
            snapshot: None,
        },
        action: "cargo:build".into(),
        desired_role: "build_runner".into(),
        mutation_authorized: true,
        proof_requirements: vec!["build_success".into()],
        release_publication: None,
        evidence_refs: vec![],
    };

    // Compose the workload envelope using the canonical protocol function
    let envelope = compose_build_agent_workload(&request, &state.config.node_id, &state.config.workspace_root)
        .map_err(|blockers| {
            (
                StatusCode::BAD_REQUEST,
                format!("composition failed: {:?}", blockers),
            )
        })?;

    // Build the durable record
    let record = AgentWorkloadRecord {
        schema: "cynic.agent_workload_record.v1".into(),
        envelope,
        state: AgentWorkloadState::Dispatched,
        claim: None,
        last_fencing_token: 0,
        cancellation_generation: 0,
        consumed_actions: 0,
        completion: None,
    };

    // Persist durable record — delivery loop will pick it up
    let _durable = state
        .store
        .compare_and_swap(&workload_id, None, &record, accepted_at_ms as u64)
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("persist failed: {}", e.code),
        ))?;

    Ok(Json(AcceptanceProof {
        schema: "cynic.builder.acceptance_proof.v1",
        workload_id,
        commit_sha,
        accepted_at_ms,
        status: "pending",
    }))
}

pub async fn run_http_server(
    config: BuilderHttpConfig,
    store: Arc<dyn AgentWorkloadStorePort>,
    listen_addr: SocketAddr,
    shutdown: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = app(config, store);
    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown.cancelled().await;
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workload_id_has_forgejo_prefix() {
        let id = format!("forgejo-{}", Uuid::new_v4());
        assert!(id.starts_with("forgejo-"));
    }
}
