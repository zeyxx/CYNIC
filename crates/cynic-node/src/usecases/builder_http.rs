//! HTTP server for the builder runtime.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use tokio_util::sync::CancellationToken;

use crate::ports::build_workload_grant_port::BuildWorkloadGrantSignerPort;
use crate::ports::build_workload_submission_port::BuildWorkloadSubmissionPort;
use crate::usecases::build_submission_orchestrator_uc::orchestrate_build_submission;
use cynic_protocol::{BuildAgentRequest, BuildSourceReference};

#[derive(Clone)]
pub struct BuilderHttpState {
    pub config: BuilderHttpConfig,
    pub signer: Arc<dyn BuildWorkloadGrantSignerPort>,
    pub submitter: Arc<dyn BuildWorkloadSubmissionPort>,
}

#[derive(Debug, Clone)]
pub struct BuilderHttpConfig {
    pub node_id: String,
    pub signing_key_path: std::path::PathBuf,
    pub state_root: std::path::PathBuf,
    pub registry_path: String,
    pub workspace_root: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ForgejoWebhookResponse {
    pub status: String,
    pub workload_id: Option<String>,
    pub message: String,
}

#[derive(Debug, serde::Serialize)]
pub struct HealthResponse {
    pub node_id: String,
    pub role: &'static str,
    pub status: &'static str,
}

pub async fn run_http_server(
    config: BuilderHttpConfig,
    signer: Arc<dyn BuildWorkloadGrantSignerPort>,
    submitter: Arc<dyn BuildWorkloadSubmissionPort>,
    listen_addr: SocketAddr,
    shutdown: CancellationToken,
) -> Result<(), String> {
    let state = BuilderHttpState { config, signer, submitter };

    let app = Router::new()
        .route("/webhook/forgejo", post(handle_forgejo_webhook))
        .route("/health", get(handle_health))
        .route("/proofs/:id", get(handle_get_proof))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(listen_addr)
        .await
        .map_err(|e| format!("bind {listen_addr}: {e}"))?;

    axum::serve(listener, app)
        .with_graceful_shutdown(async move { shutdown.cancelled().await; })
        .await
        .map_err(|e| format!("HTTP server: {e}"))
}

async fn handle_forgejo_webhook(
    State(state): State<BuilderHttpState>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<ForgejoWebhookResponse>) {
    let branch = payload.get("ref").and_then(|v| v.as_str()).unwrap_or("");
    if branch != "refs/heads/main" && branch != "refs/heads/master" {
        return (
            StatusCode::OK,
            Json(ForgejoWebhookResponse {
                status: "skipped".into(),
                workload_id: None,
                message: format!("ignored branch: {branch}"),
            }),
        );
    }

    let commit_sha = payload
        .get("commits")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if commit_sha.len() != 40 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ForgejoWebhookResponse {
                status: "error".into(),
                workload_id: None,
                message: format!("invalid commit SHA: {commit_sha}"),
            }),
        );
    }

    let repo_url = payload
        .get("repository")
        .and_then(|r| r.get("clone_url"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let default_branch = payload
        .get("repository")
        .and_then(|r| r.get("default_branch"))
        .and_then(|v| v.as_str())
        .unwrap_or("main");

    let request = BuildAgentRequest {
        schema: "cynic.node.build_agent_request.v1".into(),
        request_id: format!("forgejo-{commit_sha}"),
        requested_by: "forgejo-webhook".into(),
        source_commit: commit_sha.into(),
        source: BuildSourceReference {
            schema: "cynic.node.build_source_reference.v1".into(),
            repository_id: repo_url.into(),
            transport_uri: repo_url.into(),
            revision: commit_sha.into(),
            authority_ref: default_branch.into(),
            snapshot: None,
        },
        action: "build".into(),
        desired_role: "build_runner".into(),
        mutation_authorized: true,
        proof_requirements: vec!["build_proof".into()],
        release_publication: None,
        evidence_refs: vec![],
    };

    let now_unix_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    match orchestrate_build_submission(
        &request,
        &state.config.registry_path,
        &state.config.workspace_root,
        state.signer.as_ref(),
        state.submitter.as_ref(),
        now_unix_s,
    )
    .await
    {
        Ok(plan) => {
            let workload_id = plan.delivery.as_ref().map(|d| d.envelope.workload_id.clone());
            let status = if plan.blockers.is_empty() { "submitted" } else { "blocked" };
            (
                StatusCode::ACCEPTED,
                Json(ForgejoWebhookResponse {
                    status: status.into(),
                    workload_id,
                    message: format!("build plan: {}", plan.status),
                }),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ForgejoWebhookResponse {
                status: "error".into(),
                workload_id: None,
                message: format!("orchestrator error: {e}"),
            }),
        ),
    }
}

async fn handle_health(State(state): State<BuilderHttpState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        node_id: state.config.node_id.clone(),
        role: "build_runner",
        status: "healthy",
    })
}

async fn handle_get_proof(
    State(state): State<BuilderHttpState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let proof_path = state.config.state_root.join("proofs").join(&id);
    match tokio::fs::read_to_string(&proof_path).await {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(json) => (StatusCode::OK, Json(json)),
            Err(_) => (
                StatusCode::OK,
                Json(serde_json::json!({ "status": "raw", "content": content })),
            ),
        },
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "status": "not_found", "workload_id": id })),
        ),
    }
}
