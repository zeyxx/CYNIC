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

use cynic_protocol::{
    AGENT_CITIZENSHIP_SCHEMA, AgentBudgets, AgentCitizenshipRecord, AgentScope,
    AgentWorkloadRecord, AgentWorkloadState, BUILD_AGENT_REQUEST_SCHEMA,
    BUILD_GRANT_READ_BACK_PROOF, BUILD_SOURCE_REFERENCE_SCHEMA, BuildAgentRequest,
    BuildSourceReference, MutationAdmission, SecurityEvidenceState, admit_agent_workload,
    admit_build_mutation, compose_build_agent_workload, digest_json,
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
        .route("/workloads/submit", post(submit_workload))
        .route("/proofs/{id}", get(get_proof))
        .with_state(state)
}

/// Agent build submission payload for `POST /workloads/submit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBuildSubmission {
    pub commit_sha: String,
    #[serde(default)]
    pub repository_id: Option<String>,
    #[serde(default = "default_cargo_package")]
    pub package: Option<String>,
    #[serde(default = "default_build_action")]
    pub action: String,
    pub requested_by: String,
}

fn default_cargo_package() -> Option<String> {
    None
}

fn default_build_action() -> String {
    "check".to_string()
}

async fn submit_workload(
    State(state): State<AppState>,
    Json(payload): Json<AgentBuildSubmission>,
) -> Result<Json<AcceptanceProof>, (StatusCode, String)> {
    let commit_sha = payload.commit_sha.trim().to_string();
    if commit_sha.len() != 40 || !commit_sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "invalid commit SHA: expected 40 hex chars, got {}",
                commit_sha.len()
            ),
        ));
    }

    let accepted_at_ms = chrono::Utc::now().timestamp_millis();

    let request = BuildAgentRequest {
        schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
        request_id: format!("req-{}-{}", &commit_sha[..8], accepted_at_ms),
        requested_by: payload.requested_by.clone(),
        source_commit: commit_sha.clone(),
        source: BuildSourceReference {
            schema: BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
            repository_id: payload
                .repository_id
                .unwrap_or_else(|| state.config.repository_id.clone()),
            transport_uri: state.config.transport_uri.clone(),
            revision: commit_sha.clone(),
            authority_ref: "agent_submission".into(),
            snapshot: None,
        },
        action: match payload.action.as_str() {
            "build" => "make:build".to_string(),
            "build-release" => "make:build-release".to_string(),
            _ => "make:check".to_string(),
        },
        desired_role: "build_runner".into(),
        mutation_authorized: true,
        proof_requirements: vec![BUILD_GRANT_READ_BACK_PROOF.to_string()],
        release_publication: None,
        evidence_refs: vec![format!("agent:{}@{}", payload.requested_by, commit_sha)],
    };

    let mut envelope = compose_build_agent_workload(
        &request,
        &state.config.node_id,
        &state.config.workspace_root,
    )
    .map_err(|blockers| {
        (
            StatusCode::BAD_REQUEST,
            format!("composition failed: {blockers:?}"),
        )
    })?;

    // Do NOT wipe envelope.action.working_directory; compose_build_agent_workload
    // computes the canonical bounded action working directory and input_digest,
    // which are cryptographically sealed in plan_digest and verified by derive_build_workload_grant.

    let admission = admit_agent_workload(&envelope);
    if !admission.admitted() {
        return Err((
            StatusCode::FORBIDDEN,
            format!("workload admission refused: {:?}", admission),
        ));
    }

    let workload_id = envelope.workload_id.clone();
    if state.store.read(&workload_id).ok().flatten().is_some() {
        return Ok(Json(AcceptanceProof {
            schema: "cynic.builder.acceptance_proof.v1",
            workload_id,
            commit_sha,
            accepted_at_ms,
            status: "duplicate",
        }));
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

    if let Err(e) = state
        .store
        .compare_and_swap(&workload_id, None, &record, accepted_at_ms as u64)
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("store CAS failed: {e:?}"),
        ));
    }

    Ok(Json(AcceptanceProof {
        schema: "cynic.builder.acceptance_proof.v1",
        workload_id,
        commit_sha,
        accepted_at_ms,
        status: "accepted",
    }))
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
            let parcel_receipt =
                if record.record.state.terminal() && record.record.completion.is_some() {
                    cynic_protocol::SettledParcelReceipt::for_record(
                        &record.record,
                        record.committed_at_ms,
                    )
                    .ok()
                } else {
                    None
                };
            let proof = serde_json::json!({
                "schema": "cynic.builder.proof.v1",
                "workload_id": id,
                "state": record.record.state,
                "committed_at_ms": record.committed_at_ms,
                "record_digest": record.record_digest,
                "parcel_receipt": parcel_receipt,
                "claim": record.record.claim,
                "completion": record.record.completion,
                "envelope": record.record.envelope,
            });
            Ok(Json(proof))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn verify_webhook_signature(secret: &str, body: &Bytes, signature: &str) -> bool {
    let sig = signature.trim();
    let sig = sig.strip_prefix("sha256=").unwrap_or(sig);
    let presented = match hex::decode(sig) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };
    mac.update(body);
    mac.verify_slice(&presented).is_ok()
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
    // HMAC verification: fail-closed if webhook_secret is missing or signature fails
    let Some(secret) = &state.config.webhook_secret else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "webhook authentication unavailable: webhook_secret is not configured".into(),
        ));
    };

    let signature = headers
        .get("x-forgejo-signature")
        .or_else(|| headers.get("x-hub-signature-256"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !verify_webhook_signature(secret, &body, signature) {
        return Err((StatusCode::UNAUTHORIZED, "invalid webhook signature".into()));
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

    let request_id = forgejo_workload_id(&commit_sha);
    let accepted_at_ms = Utc::now().timestamp_millis();

    let request = BuildAgentRequest {
        schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
        request_id: request_id.clone(),
        requested_by: payload
            .pusher
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "forgejo".into()),
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

    let mut envelope = compose_build_agent_workload(
        &request,
        &state.config.node_id,
        &state.config.workspace_root,
    )
    .map_err(|blockers| {
        (
            StatusCode::BAD_REQUEST,
            format!("composition failed: {blockers:?}"),
        )
    })?;
    // The node assigns the registry-resolved layout at prepare time; the
    // gateway must not invent node paths here. The plan digest is recomputed
    // so it keeps binding the exact action stored.
    // Do NOT wipe envelope.action.working_directory; compose_build_agent_workload
    // computes the canonical bounded action working directory and input_digest,
    // which are cryptographically sealed in plan_digest and verified by derive_build_workload_grant.
    let admission = admit_agent_workload(&envelope);
    if !admission.admitted() {
        return Err((
            StatusCode::FORBIDDEN,
            format!("workload admission refused: {:?}", admission),
        ));
    }

    // The store key is the composed envelope id: `compose` derives it from
    // the request digest, so identical pushes address the same record and a
    // second delivery replays instead of duplicating.
    let workload_id = envelope.workload_id.clone();
    if state.store.read(&workload_id).ok().flatten().is_some() {
        return Ok(Json(AcceptanceProof {
            schema: "cynic.builder.acceptance_proof.v1",
            workload_id,
            commit_sha,
            accepted_at_ms,
            status: "duplicate",
        }));
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

    if let Err(error) =
        state
            .store
            .compare_and_swap(&workload_id, None, &record, accepted_at_ms as u64)
    {
        // Lost the creation race with an identical concurrent push: the
        // record now exists, so replay its acceptance instead of failing.
        if error.code == "workload_store_revision_conflict"
            && state.store.read(&workload_id).ok().flatten().is_some()
        {
            return Ok(Json(AcceptanceProof {
                schema: "cynic.builder.acceptance_proof.v1",
                workload_id,
                commit_sha,
                accepted_at_ms,
                status: "duplicate",
            }));
        }
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("persist failed: {}", error.code),
        ));
    }

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
    use uuid::Uuid;

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

    fn push_request(commit_sha: &str) -> BuildAgentRequest {
        BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: forgejo_workload_id(commit_sha),
            requested_by: "forgejo".to_string(),
            source_commit: commit_sha.to_string(),
            source: BuildSourceReference {
                schema: BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: "cynic".to_string(),
                transport_uri: "ssh://example/CYNIC.git".to_string(),
                revision: commit_sha.to_string(),
                authority_ref: "forgejo".to_string(),
                snapshot: None,
            },
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: true,
            proof_requirements: vec![BUILD_GRANT_READ_BACK_PROOF.to_string()],
            release_publication: None,
            evidence_refs: vec![format!("forgejo:push@{commit_sha}")],
        }
    }

    #[test]
    fn identical_pushes_compose_identical_workload_ids() {
        let sha = "c".repeat(40);
        let first =
            compose_build_agent_workload(&push_request(&sha), "node-1", "/tmp").expect("compose");
        let second =
            compose_build_agent_workload(&push_request(&sha), "node-1", "/tmp").expect("compose");
        assert_eq!(first.workload_id, second.workload_id);
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
    fn hmac_verification_with_sha256_prefix() {
        let secret = "test-secret";
        let body = b"{\"after\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
        assert!(verify_webhook_signature(
            secret,
            &Bytes::from_static(body),
            &expected
        ));
    }

    #[test]
    fn hmac_verification_rejects_non_hex() {
        let secret = "test-secret";
        let body = b"{\"after\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}";
        assert!(!verify_webhook_signature(
            secret,
            &Bytes::from_static(body),
            "not-a-valid-hex-digest"
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
