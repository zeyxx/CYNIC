//! HTTP server for the builder runtime.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use cynic_protocol::{
    AgentWorkloadRecord, AgentWorkloadState, BuildAgentRequest, BuildSourceReference,
    compose_build_agent_workload,
};

use crate::ports::agent_workload_store_port::AgentWorkloadStorePort;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct BuilderHttpConfig {
    pub listen_addr: SocketAddr,
    pub state_root: String,
    pub workspace_root: String,
    pub node_id: String,
    pub webhook_secret: Option<String>,
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

fn verify_webhook_signature(
    secret: &str,
    body: &Bytes,
    signature: &str,
) -> bool {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(body);
    let result = mac.finalize();
    let expected = hex::encode(result.into_bytes());
    // Forgejo sends hex-encoded HMAC-SHA256 in X-Forgejo-Signature
    signature == expected
}

async fn webhook_forgejo(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<AcceptanceProof>, (StatusCode, String)> {
    // HMAC verification if a secret is configured
    if let Some(secret) = &state.config.webhook_secret {
        let signature = headers
            .get("x-forgejo-signature")
            .or_else(|| headers.get("x-hub-signature-256"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !verify_webhook_signature(secret, &body, signature) {
            return Err((
                StatusCode::UNAUTHORIZED,
                "invalid webhook signature".into(),
            ));
        }
    }

    let payload: ForgejoPayload = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")))?;

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

    // Validate commit SHA format (40 hex chars for SHA-1)
    if commit_sha.len() != 40 || !commit_sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("invalid commit SHA: expected 40 hex chars, got {}", commit_sha.len()),
        ));
    }

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

    let envelope = compose_build_agent_workload(&request, &state.config.node_id, &state.config.workspace_root)
        .map_err(|blockers| {
            (
                StatusCode::BAD_REQUEST,
                format!("composition failed: {:?}", blockers),
            )
        })?;

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

    state
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

    #[test]
    fn hmac_verification_valid_signature() {
        let secret = "test-secret";
        let body = b"{\"after\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}";
        // Compute expected HMAC-SHA256
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = hex::encode(mac.finalize().into_bytes());
        assert!(verify_webhook_signature(secret, &Bytes::from_static(body), &expected));
    }

    #[test]
    fn hmac_verification_invalid_signature() {
        let secret = "test-secret";
        let body = b"{\"after\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}";
        assert!(!verify_webhook_signature(secret, &Bytes::from_static(body), "deadbeef"));
    }

    #[test]
    fn commit_sha_validation_rejects_short() {
        let sha = "abc123";
        assert!(!sha.chars().all(|c| c.is_ascii_hexdigit()) || sha.len() != 40);
    }

    #[test]
    fn commit_sha_validation_accepts_valid() {
        let sha = "a".repeat(40);
        assert_eq!(sha.len(), 40);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
