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
use hmac::{Hmac, Mac, digest::KeyInit};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio_util::sync::CancellationToken;

use cynic_build::{
    BUILD_AGENT_REQUEST_SCHEMA, BUILD_GRANT_READ_BACK_PROOF, BUILD_SOURCE_REFERENCE_SCHEMA,
    BuildAgentRequest, BuildSourceReference, CollisionCounter, CollisionEvent,
    COLLISION_WINDOW_MS, compose_build_agent_workload,
};
use std::sync::Mutex;
use cynic_protocol::{
    AGENT_CITIZENSHIP_SCHEMA, AgentBudgets, AgentCitizenshipRecord, AgentScope, MutationAdmission,
    SecurityEvidenceState, admit_build_mutation,
};
use cynic_workload::{AgentWorkloadRecord, AgentWorkloadState, admit_agent_workload};

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
    collisions: Arc<Mutex<CollisionCounter>>,
    evidence: cynic_host::adapters::local_evidence_store::LocalContentAddressedEvidenceStore,
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

pub fn app(
    config: BuilderHttpConfig,
    store: Arc<dyn AgentWorkloadStorePort>,
    collisions: Arc<Mutex<CollisionCounter>>,
) -> Router {
    let evidence =
        cynic_host::adapters::local_evidence_store::LocalContentAddressedEvidenceStore::new(
            std::path::PathBuf::from(&config.state_root).join("evidence"),
        );
    let state = AppState {
        config,
        store,
        collisions,
        evidence,
    };
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/webhook/forgejo", post(webhook_forgejo))
        .route("/workloads/submit", post(submit_workload))
        .route("/proofs/{id}", get(get_proof))
        .route("/records/{id}", get(get_record))
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

/// Map a CLI submit verb to its canonical build action.
///
/// Pure so the honesty rule is testable without HTTP: `check` compiles,
/// audits are explicit, unknown verbs are refused instead of silently
/// becoming whatever the old default was.
pub(crate) fn submit_action_to_canonical(action: &str) -> Result<&'static str, String> {
    match action {
        "check" => Ok("cargo:check"),
        "audit" => Ok("audit:crate"),
        "audit-all" => Ok("audit:all"),
        "build" => Ok("cargo:build"),
        "build-release" => Ok("cargo:build-release"),
        "test" => Ok("cargo:test"),
        other => Err(format!(
            "unsupported build action: {other} (expected check, audit, audit-all, build, build-release, test)"
        )),
    }
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
        branch: None,
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
        action: submit_action_to_canonical(payload.action.as_str())
            .map_err(|reason| (StatusCode::BAD_REQUEST, reason))?
            .to_string(),
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
    record_submission(&state, &workload_id, &commit_sha, accepted_at_ms as u64);
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

/// Record a submission event into the H4 collision counter (Task 5).
/// Called for every submission (accepted or duplicate) so the pure
/// `CollisionCounter` logic can determine which events collide on
/// idempotency key or source commit within the trailing 24h window.
fn record_submission(
    state: &AppState,
    idempotency_key: &str,
    source_commit: &str,
    observed_at_ms: u64,
) {
    if let Ok(mut counter) = state.collisions.lock() {
        counter.record(CollisionEvent {
            idempotency_key: idempotency_key.to_string(),
            source_commit: source_commit.to_string(),
            observed_at_ms,
        });
    }
}

/// H4 status surface (Task 5): exposes the duplicate-submit collision
/// counter over the trailing 24h window. The counter is pure logic in
/// `cynic-build`; this endpoint is the read-back surface the measurement
/// plan requires.
async fn status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let now_ms = Utc::now().timestamp_millis() as u64;
    let counter = state.collisions.lock().unwrap_or_else(|e| e.into_inner());
    Json(serde_json::json!({
        "schema": "cynic.builder.collision_status.v1",
        "generated_at_ms": now_ms,
        "window_ms": COLLISION_WINDOW_MS,
        "collisions_in_window": counter.collisions_in_window(now_ms, COLLISION_WINDOW_MS),
        "total_recorded_events": counter.total_events(),
    }))
}

async fn get_proof(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.store.read(&id) {
        Ok(Some(record)) => {
            let parcel_receipt =
                if record.record.state.terminal() && record.record.completion.is_some() {
                    cynic_workload::SettledParcelReceipt::for_record(
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

/// Read back one workload's execution record: the stored envelope plus every
/// evidence envelope its completion references, resolved digest by digest.
///
/// Unresolvable digests are explicit `null` content with the digest named in
/// `unresolved_evidence` — a record that exists is never a 404 because its
/// evidence moved. Non-terminal records report state with empty evidence.
async fn get_record(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use cynic_host::ports::evidence_store::ContentAddressedEvidenceStorePort;

    let record = match state.store.read(&id) {
        Ok(Some(record)) => record,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    let envelope = &record.record.envelope;
    let mut evidence = Vec::new();
    let mut unresolved = Vec::new();
    let completion_status = record.record.completion.as_ref().map(|completion| {
        for reference in &completion.evidence_refs {
            match state.evidence.read(&reference.digest) {
                Ok(Some(resolved)) => evidence.push(serde_json::json!({
                    "kind": reference.kind,
                    "digest": reference.digest,
                    "envelope": resolved,
                })),
                _ => {
                    unresolved.push(reference.digest.clone());
                    evidence.push(serde_json::json!({
                        "kind": reference.kind,
                        "digest": reference.digest,
                        "envelope": serde_json::Value::Null,
                    }));
                }
            }
        }
        format!("{:?}", completion.status)
    });
    // The envelope stores the real argv, never the requested verb: resolve the
    // canonical action name back from argv so the record says what ran.
    let action = cynic_build::canonical_build_action_from_argv(&envelope.action.argv);
    Ok(Json(shape_record_response(
        &id,
        &record.record.state,
        action,
        &envelope.action.argv,
        envelope.source_commit.as_deref(),
        completion_status.as_deref(),
        evidence,
        unresolved,
    )))
}

/// Pure record shaper: no store, no I/O. Tested without HTTP.
fn shape_record_response(
    workload_id: &str,
    state: &AgentWorkloadState,
    action: Option<&str>,
    argv: &[String],
    source_commit: Option<&str>,
    completion_status: Option<&str>,
    evidence: Vec<serde_json::Value>,
    unresolved_evidence: Vec<String>,
) -> serde_json::Value {
    serde_json::json!({
        "schema": "cynic.builder.record.v1",
        "workload_id": workload_id,
        "state": state,
        "action": action,
        "argv": argv,
        "source_commit": source_commit,
        "completion_status": completion_status,
        "evidence": evidence,
        "unresolved_evidence": unresolved_evidence,
    })
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
        branch: None,
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
    record_submission(&state, &workload_id, &commit_sha, accepted_at_ms as u64);
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
    let collisions = Arc::new(Mutex::new(CollisionCounter::new()));
    let app = app(config, store, collisions);
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

    use cynic_build::{
        admit_build_agent_request, canonical_argv_from_build_action,
        BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA, BuildReleasePublicationIntent,
    };

    /// Task P fix round 1: a submit verb must survive the whole chain —
    /// mapping -> composition admission -> argv construction — without a
    /// blocker. Pins the composition gate to the same action set the mapping
    /// and argv translators accept, so a narrow green that fixes only one
    /// link (e.g. mapping + argv but not the composition gate) cannot recur.
    #[test]
    fn submit_action_chains_through_admission_and_argv() {
        for verb in ["check", "build", "build-release", "audit", "audit-all", "test"] {
            let action = submit_action_to_canonical(verb).expect("known submit verb");
            let mut request = push_request(&"d".repeat(40));
            request.action = action.to_string();
            if action == "cargo:build-release" {
                // Composition requires publication intent plus the two
                // release proofs for release builds: equip the request so
                // the chain test exercises the real release path.
                request.release_publication = Some(BuildReleasePublicationIntent {
                    schema: BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA.to_string(),
                    release_id: "v9.9.9-test".to_string(),
                    release_epoch: 1,
                });
                request.proof_requirements.extend([
                    "release_binary_artifact_read_back".to_string(),
                    "signed_release_basket_read_back".to_string(),
                ]);
            }
            let admission = admit_build_agent_request(&request);
            assert!(
                !admission
                    .blockers
                    .contains(&"build_agent_request_action_not_allowed".to_string()),
                "verb {verb} -> action {action} refused by composition gate"
            );
            assert!(
                admission.blockers.is_empty(),
                "verb {verb} -> action {action} blocked: {:?}",
                admission.blockers
            );
            assert!(
                canonical_argv_from_build_action(action).is_some(),
                "action {action} must have a canonical argv"
            );
        }
    }

    #[test]
    fn record_shape_carries_resolved_and_unresolved_evidence() {
        let sha = "a".repeat(40);
        let shaped = shape_record_response(
            "wid-1",
            &AgentWorkloadState::Completed,
            Some("audit:crate"),
            &["cargo".to_string()],
            Some(sha.as_str()),
            Some("Completed"),
            vec![serde_json::json!({"kind": "k", "digest": "d", "envelope": {"exit_code": 0}})],
            vec!["missing-digest".to_string()],
        );
        assert_eq!(shaped["schema"], "cynic.builder.record.v1");
        assert_eq!(shaped["action"], "audit:crate");
        assert_eq!(shaped["evidence"][0]["envelope"]["exit_code"], 0);
        assert_eq!(shaped["unresolved_evidence"][0], "missing-digest");
    }

    #[test]
    fn record_shape_for_non_terminal_has_no_completion() {
        let shaped = shape_record_response(
            "wid-2",
            &AgentWorkloadState::Dispatched,
            None,
            &[],
            None,
            None,
            vec![],
            vec![],
        );
        assert!(shaped["completion_status"].is_null());
        assert!(shaped["evidence"].as_array().unwrap().is_empty());
    }

    #[test]
    fn submit_verbs_map_to_honest_canonical_actions() {
        assert_eq!(submit_action_to_canonical("check"), Ok("cargo:check"));
        assert_eq!(submit_action_to_canonical("audit"), Ok("audit:crate"));
        assert_eq!(submit_action_to_canonical("audit-all"), Ok("audit:all"));
        assert_eq!(submit_action_to_canonical("build"), Ok("cargo:build"));
        assert_eq!(
            submit_action_to_canonical("build-release"),
            Ok("cargo:build-release")
        );
        assert_eq!(submit_action_to_canonical("test"), Ok("cargo:test"));
        assert!(submit_action_to_canonical("deploy").is_err());
        assert!(submit_action_to_canonical("").is_err());
    }

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
            branch: None,
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
            schema: cynic_build::BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: format!("forgejo-{}", Uuid::new_v4()),
            requested_by: payload
                .pusher
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "forgejo".into()),
            source_commit: commit_sha.clone(),
            branch: None,
            source: BuildSourceReference {
                schema: cynic_build::BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: config_repo,
                transport_uri: format!(
                    "ssh://forgejo@forgejo.service.cynic:22/owner/{}.git",
                    "cynic"
                ),
                revision: commit_sha.clone(),
                authority_ref: "forgejo".into(),
                snapshot: None,
            },
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: true,
            proof_requirements: vec![cynic_build::BUILD_GRANT_READ_BACK_PROOF.to_string()],
            release_publication: None,
            evidence_refs: vec![format!("forgejo:{}@{}", reref_name(&payload), commit_sha)],
        };

        let admission = cynic_build::admit_build_agent_request(&request);
        assert_eq!(
            admission.status, "admitted",
            "blockers: {:?}",
            admission.blockers
        );

        let envelope =
            cynic_build::compose_build_agent_workload(&request, "cynic-builder", "/tmp/wf")
                .expect("webhook request composes a bounded workload");
        // Slice semantics changed deliberately: `make:check` composes the native
        // audit argv (vocabulary honesty), not the Makefile wrapper.
        assert_eq!(
            envelope.action.argv,
            vec!["cargo", "run", "-q", "-p", "cynic-cli", "--", "audit", "crate"]
        );
    }
}
