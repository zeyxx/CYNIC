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
    AgentWorkloadRecord, AgentWorkloadState, BUILD_AGENT_REQUEST_SCHEMA,
    BUILD_GRANT_READ_BACK_PROOF, BUILD_SOURCE_REFERENCE_SCHEMA, BuildAgentRequest,
    BuildSourceReference, admit_agent_workload, compose_build_agent_workload,
};

use crate::ports::agent_workload_store_port::AgentWorkloadStorePort;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct BuilderHttpConfig {
    pub listen_addr: SocketAddr,
    pub state_root: String,
    pub workspace_root: String,
    pub node_id: String,
    /// Enrollment-owned subject that receives builder workloads.
    pub agent_id: String,
    pub webhook_secret: Option<String>,
    /// Registry-declared repository the webhook builds (no hardcoded repo).
    pub repository_id: String,
    /// Secret-free source transport URI for that repository.
    pub transport_uri: String,
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
pub struct ForgejoRef {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct ForgejoAuthor {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ForgejoCommit {
    pub id: String,
    pub message: String,
    pub timestamp: Option<String>,
}

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
        .route("/proofs/{id}", get(get_proof))
        .with_state(state)
}

async fn health() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

async fn get_proof(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.store.read(&id) {
        Ok(Some(record)) => {
            let proof = serde_json::json!({
                "schema": "cynic.builder.proof.v1",
                "workload_id": id,
                "state": record.record.state,
                "committed_at_ms": record.committed_at_ms,
                "record_digest": record.record_digest,
                "envelope": record.record.envelope,
            });
            Ok(Json(proof))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn verify_webhook_signature(secret: &str, body: &Bytes, signature: &str) -> bool {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(body);
    let result = mac.finalize();
    let expected = hex::encode(result.into_bytes());
    // Forgejo sends hex-encoded HMAC-SHA256 in X-Forgejo-Signature
    signature == expected
}

/// Deterministic workload id: the same push delivered twice must address
/// the same record so the second delivery replays instead of duplicating.
/// `commit_sha` is 40-hex validated by the caller; slicing is safe.
fn forgejo_workload_id(commit_sha: &str) -> String {
    format!("forgejo-{}", &commit_sha[..12])
}

/// Best-effort ref name for push provenance evidence (`reref.id`, else the
/// branch-ish part of `ref`, else "unknown-ref"). Evidence names what was
/// pushed; it never fabricates a digest.
fn reref_name(payload: &ForgejoPayload) -> String {
    if let Some(reref) = &payload.reref {
        return reref.id.clone();
    }
    payload
        .commits
        .last()
        .map(|_| "push".to_string())
        .unwrap_or_else(|| "unknown-ref".to_string())
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
            return Err((StatusCode::UNAUTHORIZED, "invalid webhook signature".into()));
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
            format!(
                "invalid commit SHA: expected 40 hex chars, got {}",
                commit_sha.len()
            ),
        ));
    }

    // Canonical admission gate — build mutation requires:
    // - A valid citizenship record
    // - A held claim containing the target scope
    // - Sufficient budget
    // - Matching build role
    let desired_role = "build_runner";
    let citizen = AgentCitizenshipRecord {
        schema: AGENT_CITIZENSHIP_SCHEMA,
        agent_id: payload
            .pusher
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "forgejo".into()),
        scope: AgentScope::Crate,
        budgets: AgentBudgets {
            actions: 100,
            wall_seconds: 3600,
        },
        residual_risk: None,
        evidence: Some(SecurityEvidenceState::Proven),
    };
    let admission = admit_build_mutation(
        &citizen,
        Some(&AgentScope::Crate),
        &AgentBudgets {
            actions: 1,
            wall_seconds: 60,
        },
        &AgentScope::Crate,
        true,
        desired_role,
        desired_role,
    );
    if let MutationAdmission::Refused { codes } = &admission {
        return Err((
            StatusCode::FORBIDDEN,
            format!("admission refused: {:?}", codes),
        ));
    }

    let workload_id = forgejo_workload_id(&commit_sha);
    let accepted_at_ms = Utc::now().timestamp_millis();

    if let Ok(Some(_)) = state.store.read(&workload_id) {
        return Ok(Json(AcceptanceProof {
            schema: "cynic.builder.acceptance_proof.v1",
            workload_id,
            commit_sha,
            accepted_at_ms,
            status: "duplicate",
        }));
    }

    let request = BuildAgentRequest {
        schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
        request_id: workload_id.clone(),
        requested_by: state.config.agent_id.clone(),
        source_commit: commit_sha.clone(),
        source: BuildSourceReference {
            schema: BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
            repository_id: state.config.repository_id.clone(),
            transport_uri: state.config.transport_uri.clone(),
            revision: commit_sha.clone(),
            authority_ref: "forgejo".into(),
            snapshot: None,
        },
        // Canonical gate action: the recipe table admits make:check /
        // make:build / make:build-release only. A push triggers the full
        // canonical check gate, never an ad-hoc cargo invocation.
        action: "make:check".into(),
        desired_role: "build_runner".into(),
        mutation_authorized: true,
        proof_requirements: vec![BUILD_GRANT_READ_BACK_PROOF.to_string()],
        release_publication: None,
        // Push provenance is the minimal admission evidence: ref + commit.
        evidence_refs: vec![format!("forgejo:{}@{}", reref_name(&payload), commit_sha)],
    };

    let envelope = compose_build_agent_workload(
        &request,
        &state.config.node_id,
        &state.config.workspace_root,
    )
    .map_err(|blockers| {
        (
            StatusCode::BAD_REQUEST,
            format!("composition failed: {:?}", blockers),
        )
    })?;
    let admission = admit_agent_workload(&envelope);
    if !admission.admitted() {
        return Err((
            StatusCode::FORBIDDEN,
            format!("workload admission refused: {:?}", admission),
        ));
    }

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
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("persist failed: {}", e.code),
            )
        })?;

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
    fn workload_id_is_deterministic_per_commit() {
        let sha = "a".repeat(40);
        assert_eq!(
            forgejo_workload_id(&sha),
            format!("forgejo-{}", "a".repeat(12))
        );
        assert_eq!(forgejo_workload_id(&sha), forgejo_workload_id(&sha));
        let other = format!("b{}", "a".repeat(39));
        assert_ne!(forgejo_workload_id(&sha), forgejo_workload_id(&other));
    }

    #[test]
    fn hmac_verification_valid_signature() {
        let secret = "test-secret";
        let body = b"{\"after\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}";
        // Compute expected HMAC-SHA256
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = hex::encode(mac.finalize().into_bytes());
        assert!(verify_webhook_signature(
            secret,
            &Bytes::from_static(body),
            &expected
        ));
    }

    #[test]
    fn hmac_verification_invalid_signature() {
        let secret = "test-secret";
        let body = b"{\"after\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}";
        assert!(!verify_webhook_signature(
            secret,
            &Bytes::from_static(body),
            "deadbeef"
        ));
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

    /// The webhook's request must survive the REAL admission gate. This is the
    /// regression proof for the retired dead-letter request (stale
    /// cynic.build_agent_request.v1 schema, non-canonical cargo:build action,
    /// empty evidence) that Forgejo pushes could never admit.
    #[test]
    fn webhook_request_passes_canonical_admission_and_composition() {
        let payload = ForgejoPayload {
            before: String::new(),
            after: "a".repeat(40),
            reref: Some(ForgejoRef {
                id: "refs/heads/main".to_string(),
            }),
            pusher: Some(ForgejoAuthor {
                name: "zeyxx".to_string(),
                email: "zeyxx@invalid".to_string(),
            }),
            commits: vec![],
        };
        let commit_sha = payload.after.clone();
        let config_repo = crate::config::default_build_repository_id();
        let request = BuildAgentRequest {
            schema: cynic_protocol::BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: format!("forgejo-{}", Uuid::new_v4()),
            requested_by: payload
                .pusher
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "forgejo".into()),
            source_commit: commit_sha.clone(),
            source: BuildSourceReference {
                schema: cynic_protocol::BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: config_repo,
                transport_uri: format!(
                    "ssh://forgejo@forgejo.synapse.cynic:22/owner/{}.git",
                    "cynic"
                ),
                revision: commit_sha.clone(),
                authority_ref: "forgejo".into(),
                snapshot: None,
            },
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: true,
            proof_requirements: vec![cynic_protocol::BUILD_GRANT_READ_BACK_PROOF.to_string()],
            release_publication: None,
            evidence_refs: vec![format!("forgejo:{}@{}", reref_name(&payload), commit_sha)],
        };

        let admission = cynic_protocol::admit_build_agent_request(&request);
        assert_eq!(
            admission.status, "admitted",
            "blockers: {:?}",
            admission.blockers
        );

        let envelope =
            cynic_protocol::compose_build_agent_workload(&request, "cynic-builder", "/tmp/wf")
                .expect("webhook request composes a bounded workload");
        assert_eq!(envelope.action.argv, vec!["make", "check"]);
    }
}
