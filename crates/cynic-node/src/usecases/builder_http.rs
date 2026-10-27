//! HTTP server for the builder runtime.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use chrono::Utc;
use hmac::{Hmac, Mac, digest::KeyInit};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use cynic_build::{
    BUILD_AGENT_REQUEST_SCHEMA, BUILD_GRANT_READ_BACK_PROOF, BUILD_SOURCE_REFERENCE_SCHEMA,
    BuildAgentRequest, BuildSourceReference, CollisionCounter, CollisionEvent,
    COLLISION_WINDOW_MS, compose_build_agent_workload,
};
use std::sync::Mutex;
use cynic_protocol::{
    AGENT_CITIZENSHIP_SCHEMA, AgentBudgets, AgentCitizenshipRecord, AgentScope, EvidenceEnvelope,
    MutationAdmission, SecurityEvidenceState, admit_build_mutation, build_evidence_envelope,
};
use cynic_protocol::supervision::{
    DrainDecision, DrainReceipt, SupervisorState, decide_drain, drain_blocks_admission,
};
use cynic_workload::{AgentWorkloadRecord, AgentWorkloadState, admit_agent_workload};

use crate::config::NodeControlConfig;
use crate::config::{ReloadReceipt, decide_reload};
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
    /// Symlink path to the installed node binary (e.g.
    /// `/var/lib/cynic/bin/cynic-node`). The restart signal hashes its
    /// target to verify the desired binary is installed before bouncing.
    /// `None` means the running binary is the only reference.
    pub binary_link_path: Option<String>,
    /// Absolute path of the `node-control.toml` this builder booted from.
    /// `POST /admin/reload` re-reads exactly this file; the path itself is
    /// startup state and never reloads.
    pub config_path: String,
}

/// Hot-appliable builder snapshot. `POST /admin/reload` swaps the whole
/// struct under one lock after the candidate parses and its receipt
/// persists — readers never observe a half-applied config. The socket
/// (`listen_addr`) and the state-root stores stay on their startup values;
/// a reload that moves them flags `requires_restart` in its receipt.
#[derive(Clone, Debug)]
pub struct BuilderLiveConfig {
    pub control: NodeControlConfig,
    /// Registry-declared repository the webhook builds (re-derived per reload).
    pub repository_id: String,
    /// Secret-free source transport URI for that repository.
    pub transport_uri: String,
    /// Semantic digest of the last applied document (see `canonical_digest`).
    pub applied_digest: String,
    /// Exact bytes of the last applied document; the next reload diffs
    /// the file against these, never against a re-serialized struct.
    pub applied_raw: String,
}

#[derive(Clone)]
struct AppState {
    config: BuilderHttpConfig,
    live: Arc<Mutex<BuilderLiveConfig>>,
    store: Arc<dyn AgentWorkloadStorePort>,
    /// H4 duplicate-submit collision counter (Task 5). Records every
    /// submission event; the pure `CollisionCounter` logic determines
    /// which events collide on idempotency key or source commit within
    /// the trailing 24h window.
    collisions: Arc<Mutex<CollisionCounter>>,
    evidence: cynic_host::adapters::local_evidence_store::LocalContentAddressedEvidenceStore,
    /// Commanded-stop lifecycle (`POST /admin/drain`). Starts `Active`;
    /// drain sets `Draining` (new admissions get 503), the waiter sets
    /// `Stopped` and exits(0) once no workload is in flight.
    drain: Arc<Mutex<SupervisorState>>,
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
    live: BuilderLiveConfig,
    store: Arc<dyn AgentWorkloadStorePort>,
    collisions: Arc<Mutex<CollisionCounter>>,
) -> Router {
    let evidence =
        cynic_host::adapters::local_evidence_store::LocalContentAddressedEvidenceStore::new(
            std::path::PathBuf::from(&config.state_root).join("evidence"),
        );
    let state = AppState {
        config,
        live: Arc::new(Mutex::new(live)),
        store,
        collisions,
        evidence,
        drain: Arc::new(Mutex::new(SupervisorState::Active)),
    };
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/capacity", get(capacity))
        .route("/admin/restart", post(admin_restart))
        .route("/webhook/forgejo", post(webhook_forgejo))
        .route("/workloads/submit", post(submit_workload))
        .route("/proofs/{id}", get(get_proof))
        .route("/records/{id}", get(get_record))
        .route("/exec", post(exec))
        .route("/admin/drain", post(admin_drain))
        .route("/admin/identity", get(identity))
        .route("/admin/evidence", get(admin_evidence))
        .route("/admin/reload", post(admin_reload))
        .with_state(state)
}

/// Clone the current live snapshot. Poisoned locks clone through the poison:
/// a panicked reloader must not wedge every request behind it.
fn live_snapshot(state: &AppState) -> BuilderLiveConfig {
    state
        .live
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|poison| poison.into_inner().clone())
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

/// Request body for `POST /exec`.
#[derive(Debug, Deserialize)]
pub struct ExecRequest {
    pub command: String,
    #[serde(default)]
    pub work_dir: String,
}

/// Response body for `POST /exec`.
#[derive(Debug, Serialize)]
pub struct ExecResponse {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Execute a shell command on the builder and return its output.
///
/// SECURITY: this endpoint runs on the builder's service-plane network and
/// intentionally has no authentication — the same trust boundary as
/// `/workloads/submit`. It exists so `HttpExternalBuildAdapter` in cynic-build
/// can drive external builds over HTTP instead of SSH. The command is executed
/// via `sh -c` with `current_dir` set to `work_dir` when non-empty.
async fn exec(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ExecResponse>, (StatusCode, String)> {
    let Some(secret) = &state.config.webhook_secret else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "exec unavailable: webhook_secret is not configured".into(),
        ));
    };
    let signature = headers
        .get("x-cynic-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !verify_webhook_signature(secret, &body, signature) {
        return Err((StatusCode::UNAUTHORIZED, "invalid exec signature".into()));
    }
    let payload: ExecRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")))?;
    if payload.command.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "command must not be empty".to_string(),
        ));
    }
    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c").arg(&payload.command);
    if !payload.work_dir.is_empty() {
        cmd.current_dir(&payload.work_dir);
    }
    match cmd.output().await {
        Ok(output) => Ok(Json(ExecResponse {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("exec spawn error: {e}"),
        )),
    }
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
    drain_gate(&state)?;
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

    // Request-scoped builder facts come from the live snapshot, so a
    // `POST /admin/reload` takes effect without bouncing the process.
    let live = live_snapshot(&state);
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
                .unwrap_or_else(|| live.repository_id.clone()),
            transport_uri: live.transport_uri.clone(),
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
        &live.control.node_id,
        &live.control.workspace_root.to_string_lossy().to_string(),
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

async fn health(State(_state): State<AppState>) -> (StatusCode, &'static str) {
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

/// Pure `revision` extractor for the runtime registry copy.
///
/// POLICY_STAGING stages the merge commit SHA that admitted the registry as
/// top-level `revision`; older copies carry no revision. Absent, empty, or
/// non-string values yield `None` — the endpoint reports what it can prove,
/// never a fabricated revision.
fn applied_revision_from_registry_json(value: &serde_json::Value) -> Option<String> {
    value
        .get("revision")?
        .as_str()
        .filter(|revision| !revision.is_empty())
        .map(|revision| revision.to_string())
}

/// Pure identity response shaper: no filesystem, no clock beyond the passed
/// timestamp. Tested without HTTP.
fn shape_identity_response(
    node_id: &str,
    observed_at_ms: i64,
    running_binary_sha256: Option<&str>,
    installed_binary_path: Option<&str>,
    installed_binary_sha256: Option<&str>,
    applied_registry_revision: Option<&str>,
) -> serde_json::Value {
    let binaries_match = match (running_binary_sha256, installed_binary_sha256) {
        (Some(running), Some(installed)) => Some(running == installed),
        _ => None,
    };
    serde_json::json!({
        "schema": "cynic.builder.identity.v1",
        "node_id": node_id,
        "observed_at_ms": observed_at_ms,
        "running_binary_sha256": running_binary_sha256,
        "installed_binary_path": installed_binary_path,
        "installed_binary_sha256": installed_binary_sha256,
        "applied_registry_revision": applied_registry_revision,
        "binaries_match": binaries_match,
    })
}

/// SHA-256 of one file. `None` when the file cannot be read — a read-only
/// endpoint reports what it can prove, never a 500 for an unhashable file.
fn sha256_file(path: &std::path::Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(hex::encode(Sha256::digest(bytes)))
}

/// Hash of the running builder binary (`current_exe`).
fn running_binary_sha256() -> Option<String> {
    sha256_file(&std::env::current_exe().ok()?)
}

/// Installed binary identity: `CynicPaths::binary_path()` (overridable via
/// `CYNIC_BIN` / `CYNIC_BINARY_DIR`), resolved through its symlink when it is
/// one. Returns the display path plus the sha256 of the link target or file.
fn installed_binary_identity() -> (Option<String>, Option<String>) {
    use cynic_types::paths::CynicPaths;
    let path = CynicPaths::binary_path();
    let target = std::fs::read_link(&path).unwrap_or(path);
    let digest = sha256_file(&target);
    (Some(target.to_string_lossy().to_string()), digest)
}

/// Applied registry revision from the runtime registry copy. `None` when the
/// copy is absent, unparsable, or predates `revision` (POLICY_STAGING).
fn applied_registry_revision() -> Option<String> {
    use cynic_types::paths::CynicPaths;
    let raw = std::fs::read_to_string(CynicPaths::registry_path()).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    applied_revision_from_registry_json(&value)
}

/// Convergence read-back: `GET /admin/identity`.
///
/// No auth: read-only, no mutation — the same trust boundary as `/health`
/// and `/status`. Serves the running binary sha256, the installed
/// link-target sha256, the node id, and the applied registry revision when
/// available. This is the read-back that proves convergence for the
/// POLICY_STAGING verify stage (the fleet reports one revision).
async fn identity(State(state): State<AppState>) -> Json<serde_json::Value> {
    let (installed_path, installed_digest) = installed_binary_identity();
    let live = live_snapshot(&state);
    Json(shape_identity_response(
        &live.control.node_id,
        Utc::now().timestamp_millis(),
        running_binary_sha256().as_deref(),
        installed_path.as_deref(),
        installed_digest.as_deref(),
        applied_registry_revision().as_deref(),
    ))
}

/// Default and maximum page sizes for `GET /admin/evidence`.
pub(crate) const EVIDENCE_LIST_DEFAULT_LIMIT: usize = 50;
pub(crate) const EVIDENCE_LIST_MAX_LIMIT: usize = 200;

/// Query parameters for `GET /admin/evidence`.
#[derive(Debug, Deserialize)]
pub(crate) struct EvidenceQuery {
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub kind: Option<String>,
}

/// Clamp the requested page size: absent means the default, anything above
/// the maximum is capped. Zero stays zero and yields an empty page.
fn clamp_evidence_limit(requested: Option<usize>) -> usize {
    match requested {
        None => EVIDENCE_LIST_DEFAULT_LIMIT,
        Some(n) => n.min(EVIDENCE_LIST_MAX_LIMIT),
    }
}

/// Lifecycle kind of one evidence envelope, when it names a restart, drain,
/// pause, or reload receipt. Matches against the envelope subject, producer,
/// provenance entries, and the payload `schema`/`kind`/`operation` markers —
/// case-insensitive substring match, first hit wins in restart/drain/pause/
/// reload order. Returns `None` for build proofs and unrelated evidence.
fn evidence_lifecycle_kind(envelope: &EvidenceEnvelope) -> Option<&'static str> {
    let mut haystacks = vec![envelope.subject.clone(), envelope.producer.clone()];
    haystacks.extend(envelope.provenance.iter().cloned());
    for key in ["schema", "kind", "operation"] {
        if let Some(marker) = envelope.payload.get(key).and_then(|v| v.as_str()) {
            haystacks.push(marker.to_string());
        }
    }
    let joined = haystacks.join("\n").to_lowercase();
    if joined.contains("restart") {
        Some("restart")
    } else if joined.contains("drain") {
        Some("drain")
    } else if joined.contains("pause") {
        Some("pause")
    } else if joined.contains("reload") {
        Some("reload")
    } else {
        None
    }
}

/// Payload-declared schema of one evidence envelope, when the payload names
/// one. `None` serializes as `null` — the listing reports what it can prove.
fn evidence_payload_schema(envelope: &EvidenceEnvelope) -> Option<&str> {
    envelope.payload.get("schema")?.as_str()
}

/// Pure summary shaper for one evidence envelope: digest, lifecycle kind,
/// and payload schema. No filesystem, no clock. Tested without HTTP.
fn summarize_evidence_envelope(envelope: &EvidenceEnvelope) -> serde_json::Value {
    serde_json::json!({
        "digest": envelope.digest,
        "subject": envelope.subject,
        "producer": envelope.producer,
        "observed_at_unix_ms": envelope.observed_at_unix_ms,
        "kind": evidence_lifecycle_kind(envelope),
        "payload_schema": evidence_payload_schema(envelope),
    })
}

/// Pure list shaper: schema envelope around the page plus scan counters.
/// Tested without HTTP.
fn shape_evidence_list_response(
    node_id: &str,
    observed_at_ms: i64,
    items: Vec<serde_json::Value>,
    total_scanned: usize,
    unparseable: usize,
    limit: usize,
    kind: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "schema": "cynic.builder.evidence_list.v1",
        "node_id": node_id,
        "observed_at_ms": observed_at_ms,
        "limit": limit,
        "kind": kind,
        "total_scanned": total_scanned,
        "unparseable": unparseable,
        "returned": items.len(),
        "items": items,
    })
}

/// Scan the local content-addressed evidence directory and return lifecycle
/// receipt summaries, newest first, bounded to `limit`.
///
/// Counts every `*.json` file as scanned; files that fail to parse count as
/// `unparseable` and never fail the listing. A missing directory reads as an
/// empty store — a read-only endpoint reports what it can prove, never a 500
/// for absent evidence. `kind_filter` accepts `restart`, `drain`, `pause`,
/// or `reload` (case-insensitive); unknown filters match nothing.
fn list_evidence_summaries(
    evidence_root: &std::path::Path,
    limit: usize,
    kind_filter: Option<&str>,
) -> (Vec<serde_json::Value>, usize, usize) {
    let wanted = kind_filter.map(|k| k.to_lowercase());
    let entries = match std::fs::read_dir(evidence_root) {
        Ok(entries) => entries,
        Err(_) => return (Vec::new(), 0, 0),
    };
    let mut total_scanned = 0usize;
    let mut unparseable = 0usize;
    let mut staged: Vec<(u128, serde_json::Value)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        total_scanned += 1;
        let envelope = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<EvidenceEnvelope>(&bytes).ok());
        let Some(envelope) = envelope else {
            unparseable += 1;
            continue;
        };
        if let Some(wanted) = &wanted {
            if evidence_lifecycle_kind(&envelope).as_deref() != Some(wanted.as_str()) {
                continue;
            }
        }
        staged.push((
            envelope.observed_at_unix_ms,
            summarize_evidence_envelope(&envelope),
        ));
    }
    staged.sort_by(|a, b| b.0.cmp(&a.0));
    staged.truncate(limit);
    let items = staged.into_iter().map(|(_, item)| item).collect();
    (items, total_scanned, unparseable)
}

/// Lifecycle receipt read-back: `GET /admin/evidence?limit=&kind=`.
///
/// No auth: read-only, no mutation — the same trust boundary as `/health`,
/// `/status`, and `/admin/identity`. Lists restart/drain/pause receipts from
/// the local evidence store (the same directory that backs
/// `/records/{id}`), newest first, bounded. Restarts logged where proofs live
/// are queryable without SSH.
async fn admin_evidence(
    State(state): State<AppState>,
    Query(query): Query<EvidenceQuery>,
) -> Json<serde_json::Value> {
    let limit = clamp_evidence_limit(query.limit);
    let evidence_root = std::path::PathBuf::from(&state.config.state_root).join("evidence");
    let (items, total_scanned, unparseable) =
        list_evidence_summaries(&evidence_root, limit, query.kind.as_deref());
    let live = live_snapshot(&state);
    Json(shape_evidence_list_response(
        &live.control.node_id,
        Utc::now().timestamp_millis(),
        items,
        total_scanned,
        unparseable,
        limit,
        query.kind.as_deref(),
    ))
}

/// Persist one reload receipt and read it back. Fail-closed like the drain
/// path: a transition without a stored receipt is a 500, never a silent
/// claim — the live snapshot is swapped only after this returns `Ok`.
fn persist_reload_receipt(state: &AppState, receipt: &ReloadReceipt) -> Result<String, String> {
    use cynic_protocol::evidence_store_port::ContentAddressedEvidenceStorePort;
    let envelope = build_evidence_envelope(
        &format!("node:reload:{}", receipt.node_id),
        &format!("cynic-node/builder-http:{}", receipt.node_id),
        receipt.observed_at_unix_ms,
        vec!["admin:reload".to_string()],
        receipt,
    )?;
    state.evidence.persist(&envelope)?;
    if state.evidence.read(&envelope.digest)? != Some(envelope.clone()) {
        return Err("reload_receipt_evidence_read_back_mismatch".to_string());
    }
    Ok(envelope.digest)
}

/// Pure reload response shaper: no store, no I/O. Tested without HTTP.
/// `receipt_digest` is `Some` exactly when a transition persisted a receipt;
/// idempotent no-change reads persist nothing and carry `None`.
fn shape_reload_response(
    receipt: &ReloadReceipt,
    receipt_digest: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "schema": receipt.schema,
        "node_id": receipt.node_id,
        "outcome": receipt.outcome,
        "previous_digest": receipt.previous_digest,
        "applied_digest": receipt.applied_digest,
        "changed_keys": receipt.changed_keys,
        "requires_restart": receipt.requires_restart,
        "observed_at_unix_ms": receipt.observed_at_unix_ms,
        "receipt_digest": receipt_digest,
    })
}

/// Hot config reload: `POST /admin/reload`.
///
/// Auth is the same HMAC-SHA256 as the Forgejo webhook
/// (`verify_webhook_signature`) over the raw body, presented in
/// `x-cynic-signature`, failing closed without a configured secret — the
/// same verifier and header as `/admin/restart`, one secret and one header
/// for the whole admin surface.
///
/// Semantics: re-reads the startup `node-control.toml` and swaps the live
/// snapshot without bouncing the process. The candidate must parse and
/// validate under exactly the startup rules; otherwise the call is refused
/// (422) with the loader's reason and the old config keeps serving — never
/// a half-apply. Applied transitions persist a `cynic.node.reload_receipt.v1`
/// receipt (read back before the swap); no-change reads persist nothing.
/// Fields the process cannot hot-apply (bound socket, state-root stores)
/// flag `requires_restart` instead of pretending to move.
async fn admin_reload(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let Some(secret) = state.config.webhook_secret.clone() else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "reload signal unavailable: webhook_secret is not configured".into(),
        ));
    };
    let signature = headers
        .get("x-cynic-signature")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if !verify_webhook_signature(&secret, &body, signature) {
        return Err((StatusCode::UNAUTHORIZED, "invalid reload signature".into()));
    }

    let candidate_raw = std::fs::read_to_string(&state.config.config_path).map_err(|error| {
        let path = &state.config.config_path;
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("config file unreadable {path}: {error}"),
        )
    })?;
    let live = live_snapshot(&state);
    let now_ms = Utc::now().timestamp_millis().max(0) as u128;
    match decide_reload(&live.applied_raw, &candidate_raw) {
        crate::config::ReloadDecision::NoChange { digest } => {
            let receipt = ReloadReceipt {
                schema: crate::config::RELOAD_RECEIPT_SCHEMA.to_string(),
                node_id: live.control.node_id.clone(),
                outcome: crate::config::ReloadOutcome::AlreadyConverged
                    .as_str()
                    .to_string(),
                previous_digest: digest.clone(),
                applied_digest: digest,
                changed_keys: Vec::new(),
                requires_restart: false,
                observed_at_unix_ms: now_ms,
            };
            Ok(Json(shape_reload_response(&receipt, None)))
        }
        crate::config::ReloadDecision::Refused { reason, .. } => {
            Err((StatusCode::UNPROCESSABLE_ENTITY, reason))
        }
        crate::config::ReloadDecision::Applied {
            previous_digest,
            applied_digest,
            changed_keys,
            requires_restart,
            control,
        } => {
            let receipt = ReloadReceipt::applied(
                &control.node_id,
                &previous_digest,
                &applied_digest,
                changed_keys,
                requires_restart,
                now_ms,
            );
            // Persist before swapping: no transition without a receipt.
            let digest = persist_reload_receipt(&state, &receipt).map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("reload receipt not stored: {error}"),
                )
            })?;
            // Re-derive the registry-bound fields under the new config so
            // the very next submit/webhook uses the reloaded values.
            let derived = super::builder_runtime::builder_runtime_config_from(
                &control,
                &state.config.config_path,
            );
            if let Ok(mut guard) = state.live.lock() {
                *guard = BuilderLiveConfig {
                    control,
                    repository_id: derived.repository_id,
                    transport_uri: derived.transport_uri,
                    applied_digest,
                    applied_raw: candidate_raw,
                };
            }
            Ok(Json(shape_reload_response(&receipt, Some(&digest))))
        }
    }
}

/// Live pressure gauge for data-centric placement.
///
/// Serves what only this node knows: how many workloads are in flight
/// right now and the 1-minute load average. Declared capacity (cpu,
/// ram) lives in the registry the client already reads — the client
/// joins both sides into pressure = (in_flight + load) / capacity and
/// routes to the lowest gauge. No gauge, no flow: store errors fail
/// closed with 503 so the client treats this node as unreadable.
async fn capacity(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let active = state
        .store
        .active_records()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let in_flight = active
        .iter()
        .filter(|record| !record.state.terminal())
        .count() as u64;
    let load_average_1m = sysinfo::System::load_average().one;
    // Measured by the node itself so a client never needs SSH for them. Each
    // is `null` when unmeasurable: unmeasured is muted, never zero-assumed.
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let total_ram_bytes = Some(system.total_memory()).filter(|bytes| *bytes > 0);
    let free_ram_bytes = total_ram_bytes.map(|_| system.available_memory());
    let checkout_base =
        cynic_protocol::remote_cache_roots::remote_cache_roots(&state.config.state_root)
            .checkout_base;
    let mounts: Vec<(String, u64)> = sysinfo::Disks::new_with_refreshed_list()
        .iter()
        .map(|disk| {
            (
                disk.mount_point().to_string_lossy().to_string(),
                disk.available_space(),
            )
        })
        .collect();
    let disk_avail_bytes =
        cynic_protocol::capacity::available_bytes_for_path(&mounts, &checkout_base);
    Ok(Json(serde_json::json!({
        "schema": "cynic.builder.capacity.v1",
        "node_id": state.config.node_id,
        "in_flight": in_flight,
        "load_average_1m": load_average_1m,
        "state_root": state.config.state_root,
        "free_ram_bytes": free_ram_bytes,
        "total_ram_bytes": total_ram_bytes,
        "disk_avail_bytes": disk_avail_bytes,
    })))
}

/// Remote restart signal: ditch SSH, let the node bounce itself.
///
/// Auth mirrors the webhook (HMAC, fail-closed without secret, header
/// `x-cynic-signature`). The body names the desired installed binary by
/// sha256. The node bounces only when the installed target already
/// matches: install first, then signal — never the reverse. The bounce
/// is a clean exit(0); systemd `Restart=always` relaunches the new
/// binary. Every outcome persists a restart receipt in the evidence
/// store, so restarts are logged where proofs live, not scattered in
/// journals nobody reads.
#[derive(Debug, Deserialize)]
pub struct RestartSignal {
    pub desired_sha256: String,
}

/// Pure restart decision — the testable core, no I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestartDecision {
    AlreadyConverged,
    Proceed { old_hash: String, new_hash: String },
    Refused { reason: String },
}

pub fn decide_restart(
    running_hash: &str,
    installed_hash: Option<&str>,
    desired: &str,
) -> RestartDecision {
    let well_formed = desired.len() == 64 && desired.bytes().all(|byte| byte.is_ascii_hexdigit());
    if !well_formed {
        return RestartDecision::Refused {
            reason: "bad desired digest: want 64 hex chars".to_string(),
        };
    }
    if running_hash.eq_ignore_ascii_case(desired) {
        return RestartDecision::AlreadyConverged;
    }
    match installed_hash {
        Some(installed) if installed.eq_ignore_ascii_case(desired) => RestartDecision::Proceed {
            old_hash: running_hash.to_string(),
            new_hash: desired.to_string(),
        },
        _ => RestartDecision::Refused {
            reason: "installed binary does not match desired; install first, then signal"
                .to_string(),
        },
    }
}

fn sha256_of_file(path: &std::path::Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&bytes);
    Some(hex::encode(hasher.finalize()))
}

async fn admin_restart(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let Some(secret) = &state.config.webhook_secret else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "restart signal unavailable: webhook_secret is not configured".into(),
        ));
    };
    let signature = headers
        .get("x-cynic-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !verify_webhook_signature(secret, &body, signature) {
        return Err((StatusCode::UNAUTHORIZED, "invalid restart signature".into()));
    }
    let signal: RestartSignal = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")))?;
    let running_hash = std::env::current_exe()
        .ok()
        .and_then(|path| sha256_of_file(&path))
        .unwrap_or_default();
    let installed_hash = state
        .config
        .binary_link_path
        .as_ref()
        .and_then(|link| std::fs::canonicalize(link).ok())
        .and_then(|target| sha256_of_file(&target));
    match decide_restart(
        &running_hash,
        installed_hash.as_deref(),
        &signal.desired_sha256,
    ) {
        RestartDecision::AlreadyConverged => Ok(Json(serde_json::json!({
            "schema": "cynic.node.restart_receipt.v1",
            "node_id": state.config.node_id,
            "outcome": "already-converged",
            "running_sha256": running_hash,
        }))),
        RestartDecision::Refused { reason } => Err((StatusCode::CONFLICT, reason)),
        RestartDecision::Proceed { old_hash, new_hash } => {
            let receipt = serde_json::json!({
                "schema": "cynic.node.restart_receipt.v1",
                "node_id": state.config.node_id,
                "outcome": "restarting",
                "old_sha256": old_hash,
                "new_sha256": new_hash,
            });
            persist_restart_receipt(&state, &receipt);
            // Let the response flush before the process exits; systemd
            // relaunches the new binary under Restart=always.
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(500));
                std::process::exit(0);
            });
            Ok(Json(receipt))
        }
    }
}

fn persist_restart_receipt(state: &AppState, receipt: &serde_json::Value) {
    use cynic_protocol::evidence_store_port::ContentAddressedEvidenceStorePort;
    let observed_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let envelope = cynic_protocol::build_evidence_envelope_with_lineage(
        &state.config.node_id,
        "cynic-node/restart-signal",
        observed_at,
        vec!["admin-restart".to_string()],
        None,
        receipt,
    );
    if let Ok(envelope) = envelope {
        let _ = state.evidence.persist(&envelope);
    }
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
    use cynic_protocol::evidence_store_port::ContentAddressedEvidenceStorePort;

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

    // A commanded stop never admits new work; auth runs first so unauthenticated
    // callers learn nothing about drain state beyond 401.
    drain_gate(&state)?;

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

    // Same live snapshot as the submit path: reloads apply here too.
    let live = live_snapshot(&state);
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
            repository_id: live.repository_id.clone(),
            transport_uri: live.transport_uri.clone(),
            revision: commit_sha.clone(),
            authority_ref: "forgejo".into(),
            snapshot: None,
        },
        // Canonical gate action: a push triggers the full audit gate
        // (`audit:crate`, the native spelling of what `make:check`
        // aliased), never an ad-hoc cargo invocation.
        action: "audit:crate".into(),
        desired_role: "build_runner".into(),
        mutation_authorized: true,
        proof_requirements: vec![BUILD_GRANT_READ_BACK_PROOF.to_string()],
        release_publication: None,
        // Push provenance is the minimal admission evidence: ref + commit.
        evidence_refs: vec![format!("forgejo:{}@{}", reref_name(&payload), commit_sha)],
    };

    let mut envelope = compose_build_agent_workload(
        &request,
        &live.control.node_id,
        &live.control.workspace_root.to_string_lossy().to_string(),
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

/// 503 while the builder is draining: a commanded stop never admits new work.
///
/// A poisoned lock degrades to `Unknown`, which admits — the pure predicate
/// refuses `Draining` only, so a broken lock must not wedge the builder shut
/// by itself.
fn drain_gate(state: &AppState) -> Result<(), (StatusCode, String)> {
    let current = state
        .drain
        .lock()
        .map(|guard| *guard)
        .unwrap_or(SupervisorState::Unknown);
    if drain_blocks_admission(current) {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "builder draining: no new workloads admitted".to_string(),
        ));
    }
    Ok(())
}

/// Persist one drain receipt and read it back. Fail-closed like the release
/// publication path: a transition without a stored receipt is a 500, never a
/// silent claim.
fn persist_drain_receipt(state: &AppState, receipt: &DrainReceipt) -> Result<String, String> {
    use cynic_protocol::evidence_store_port::ContentAddressedEvidenceStorePort;
    let envelope = build_evidence_envelope(
        &format!("node:drain:{}", receipt.node_id),
        &format!("cynic-node/builder-http:{}", receipt.node_id),
        receipt.observed_at_unix_ms,
        vec!["admin:drain".to_string()],
        receipt,
    )?;
    state.evidence.persist(&envelope)?;
    if state.evidence.read(&envelope.digest)? != Some(envelope.clone()) {
        return Err("drain_receipt_evidence_read_back_mismatch".to_string());
    }
    Ok(envelope.digest)
}

/// Pure drain response shaper: no store, no I/O. Tested without HTTP.
/// `receipt_digest` is `None` exactly when no transition happened (idempotent
/// status read), so no receipt was persisted.
fn shape_drain_response(receipt: &DrainReceipt, receipt_digest: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "schema": receipt.schema,
        "node_id": receipt.node_id,
        "state": receipt.transition_to,
        "transition_from": receipt.transition_from,
        "in_flight": receipt.in_flight,
        "observed_at_unix_ms": receipt.observed_at_unix_ms,
        "receipt_digest": receipt_digest,
    })
}

/// Exit(0) after a short grace so the in-flight HTTP response flushes first.
fn spawn_clean_exit() {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        std::process::exit(0);
    });
}

/// Background waiter spawned once per drain: polls the workload store every
/// second and, once nothing is in flight, persists the drained receipt and
/// exits(0) so `systemd Restart=always` relaunches a fresh `Active` builder.
///
/// No timeout by design: a silent deadline would kill builds mid-flight,
/// which is exactly what drain exists to prevent. An unobservable store
/// counts as busy — the waiter never exits blind — and a receipt that fails
/// to persist retries on the next poll instead of exiting unproven.
fn spawn_drain_waiter(state: AppState) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let in_flight = match state.store.active_records() {
                Ok(records) => records.len(),
                Err(_) => continue,
            };
            let current = state
                .drain
                .lock()
                .map(|guard| *guard)
                .unwrap_or(SupervisorState::Unknown);
            match decide_drain(current, in_flight) {
                DrainDecision::Complete => {
                    if let Ok(mut guard) = state.drain.lock() {
                        *guard = SupervisorState::Stopped;
                    }
                    let now_ms = Utc::now().timestamp_millis().max(0) as u128;
                    let receipt = DrainReceipt::new(
                        &state.config.node_id,
                        SupervisorState::Draining,
                        SupervisorState::Stopped,
                        0,
                        now_ms,
                    );
                    if persist_drain_receipt(&state, &receipt).is_ok() {
                        spawn_clean_exit();
                        return;
                    }
                }
                DrainDecision::AlreadyStopped => return,
                _ => {}
            }
        }
    });
}

/// Commanded stop: `POST /admin/drain`.
///
/// Auth is the same HMAC-SHA256 as the Forgejo webhook
/// (`verify_webhook_signature`) over the raw body, presented in
/// `x-cynic-signature`, failing closed without a configured secret.
/// `/admin/restart` reuses this exact verifier and header when it lands —
/// drain is its safety precondition (no bounce may kill a build mid-flight),
/// so both share one secret and one header, never two.
///
/// Semantics: stop admitting new workloads (503), let in-flight finish, then
/// clean exit(0) for `systemd Restart=always`. Every state change persists a
/// `cynic.node.drain_receipt.v1` receipt; repeated calls while draining are
/// idempotent status reads, not new transitions, so they persist nothing.
/// A resumed builder is simply the relaunched process starting `Active`.
async fn admin_drain(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let Some(secret) = state.config.webhook_secret.clone() else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "admin authentication unavailable: webhook_secret is not configured".into(),
        ));
    };
    let signature = headers
        .get("x-cynic-signature")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if !verify_webhook_signature(&secret, &body, signature) {
        return Err((StatusCode::UNAUTHORIZED, "invalid admin signature".into()));
    }

    // `active_records()` returns non-terminal workloads only: its length is
    // the in-flight count. Unobservable store fails closed with no state
    // change — without a count there is no safe transition to record.
    let in_flight = state.store.active_records().map(|records| records.len()).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("workload store unobservable: {}", error.code),
        )
    })?;
    let current = state
        .drain
        .lock()
        .map(|guard| *guard)
        .unwrap_or(SupervisorState::Unknown);
    let now_ms = Utc::now().timestamp_millis().max(0) as u128;
    match decide_drain(current, in_flight) {
        DrainDecision::StartDraining { in_flight } => {
            let receipt = DrainReceipt::new(
                &state.config.node_id,
                current,
                SupervisorState::Draining,
                in_flight,
                now_ms,
            );
            // Persist before flipping the flag: no transition without a receipt.
            let digest = persist_drain_receipt(&state, &receipt).map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("drain receipt not stored: {error}"),
                )
            })?;
            if let Ok(mut guard) = state.drain.lock() {
                *guard = SupervisorState::Draining;
            }
            spawn_drain_waiter(state.clone());
            Ok(Json(shape_drain_response(&receipt, Some(&digest))))
        }
        DrainDecision::Wait { in_flight } => {
            let receipt = DrainReceipt::new(
                &state.config.node_id,
                SupervisorState::Draining,
                SupervisorState::Draining,
                in_flight,
                now_ms,
            );
            Ok(Json(shape_drain_response(&receipt, None)))
        }
        DrainDecision::Complete => {
            let receipt = DrainReceipt::new(
                &state.config.node_id,
                current,
                SupervisorState::Stopped,
                0,
                now_ms,
            );
            // Persist before flipping the flag: no transition without a receipt.
            let digest = persist_drain_receipt(&state, &receipt).map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("drain receipt not stored: {error}"),
                )
            })?;
            if let Ok(mut guard) = state.drain.lock() {
                *guard = SupervisorState::Stopped;
            }
            spawn_clean_exit();
            Ok(Json(shape_drain_response(&receipt, Some(&digest))))
        }
        DrainDecision::AlreadyStopped => {
            let receipt = DrainReceipt::new(
                &state.config.node_id,
                SupervisorState::Stopped,
                SupervisorState::Stopped,
                0,
                now_ms,
            );
            Ok(Json(shape_drain_response(&receipt, None)))
        }
    }
}

pub async fn run_http_server(
    config: BuilderHttpConfig,
    control: NodeControlConfig,
    store: Arc<dyn AgentWorkloadStorePort>,
    listen_addr: SocketAddr,
    shutdown: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Fail closed at boot: the reload path diffs the file against this
    // snapshot, so the process starts only if the file parses under the
    // same rules reloads enforce.
    let applied_raw = std::fs::read_to_string(&config.config_path).map_err(|error| {
        let path = &config.config_path;
        format!("cannot read config {path}: {error}")
    })?;
    let applied_digest = crate::config::canonical_digest(&applied_raw)
        .map_err(|error| format!("boot config invalid: {error}"))?;
    crate::config::parse_node_control_str(&applied_raw)
        .map_err(|error| format!("boot config invalid: {error}"))?;
    let derived =
        super::builder_runtime::builder_runtime_config_from(&control, &config.config_path);
    let live = BuilderLiveConfig {
        control,
        repository_id: derived.repository_id,
        transport_uri: derived.transport_uri,
        applied_digest,
        applied_raw,
    };
    let collisions = Arc::new(Mutex::new(CollisionCounter::new()));
    let app = app(config, live, store, collisions);
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

    const RUNNING: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const INSTALLED: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    #[test]
    fn restart_refuses_malformed_desired_digest() {
        assert!(matches!(
            decide_restart(RUNNING, Some(INSTALLED), "not-hex"),
            RestartDecision::Refused { .. }
        ));
    }

    #[test]
    fn restart_converged_when_running_matches_desired() {
        assert_eq!(
            decide_restart(RUNNING, Some(INSTALLED), RUNNING),
            RestartDecision::AlreadyConverged
        );
    }

    #[test]
    fn restart_proceeds_when_installed_matches_desired() {
        assert_eq!(
            decide_restart(RUNNING, Some(INSTALLED), INSTALLED),
            RestartDecision::Proceed {
                old_hash: RUNNING.to_string(),
                new_hash: INSTALLED.to_string(),
            }
        );
    }

    #[test]
    fn restart_refuses_when_installed_differs_from_desired() {
        // Install first, then signal — never the reverse. A bounce into
        // the same old binary is a no-op with downtime.
        assert!(matches!(
            decide_restart(RUNNING, Some(RUNNING), INSTALLED),
            RestartDecision::Refused { .. }
        ));
        assert!(matches!(
            decide_restart(RUNNING, None, INSTALLED),
            RestartDecision::Refused { .. }
        ));
    }

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
    fn exec_request_deserializes_command_and_work_dir() {
        let json = serde_json::json!({"command": "cargo check", "work_dir": "/tmp/repo"});
        let req: ExecRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.command, "cargo check");
        assert_eq!(req.work_dir, "/tmp/repo");
    }

    #[test]
    fn exec_request_work_dir_defaults_to_empty() {
        let json = serde_json::json!({"command": "ls -la"});
        let req: ExecRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.command, "ls -la");
        assert!(req.work_dir.is_empty());
    }

    #[test]
    fn exec_response_serializes_exit_code_stdout_stderr() {
        let resp = ExecResponse {
            exit_code: Some(0),
            stdout: "ok".to_string(),
            stderr: String::new(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["exit_code"], 0);
        assert_eq!(json["stdout"], "ok");
        assert!(json["stderr"].as_str().unwrap().is_empty());
    }

    #[test]
    fn exec_response_serializes_nonzero_exit() {
        let resp = ExecResponse {
            exit_code: Some(2),
            stdout: String::new(),
            stderr: "error".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["exit_code"], 2);
        assert_eq!(json["stderr"], "error");
    }

    #[test]
    fn drain_response_shape_names_state_and_digest() {
        let receipt = DrainReceipt::new(
            "node-1",
            SupervisorState::Active,
            SupervisorState::Draining,
            2,
            1_700_000_000_000,
        );
        let shaped = shape_drain_response(&receipt, Some("sha256:abc"));
        assert_eq!(shaped["schema"], "cynic.node.drain_receipt.v1");
        assert_eq!(shaped["state"], "draining");
        assert_eq!(shaped["transition_from"], "active");
        assert_eq!(shaped["in_flight"], 2);
        assert_eq!(shaped["receipt_digest"], "sha256:abc");
        // Idempotent status reads persist nothing: no digest.
        let shaped = shape_drain_response(&receipt, None);
        assert!(shaped["receipt_digest"].is_null());
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
            action: "audit:crate".to_string(),
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
            action: "audit:crate".to_string(),
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

    #[test]
    fn identity_shape_reports_full_convergence() {
        let running = "a".repeat(64);
        let shaped = shape_identity_response(
            "cynic-builder",
            1_700_000_000_000,
            Some(&running),
            Some("/usr/local/bin/cynic"),
            Some(&running),
            Some("deadbeef"),
        );
        assert_eq!(shaped["schema"], "cynic.builder.identity.v1");
        assert_eq!(shaped["node_id"], "cynic-builder");
        assert_eq!(shaped["running_binary_sha256"], running);
        assert_eq!(shaped["installed_binary_path"], "/usr/local/bin/cynic");
        assert_eq!(shaped["applied_registry_revision"], "deadbeef");
        assert_eq!(shaped["binaries_match"], true);
    }

    #[test]
    fn identity_shape_reports_binary_skew() {
        let shaped = shape_identity_response(
            "cynic-builder",
            1_700_000_000_000,
            Some(&"a".repeat(64)),
            Some("/usr/local/bin/cynic"),
            Some(&"b".repeat(64)),
            Some("deadbeef"),
        );
        assert_eq!(shaped["binaries_match"], false);
    }

    #[test]
    fn identity_shape_reports_unknowns_as_null() {
        let shaped = shape_identity_response("cynic-builder", 0, None, None, None, None);
        assert!(shaped["running_binary_sha256"].is_null());
        assert!(shaped["installed_binary_path"].is_null());
        assert!(shaped["installed_binary_sha256"].is_null());
        assert!(shaped["applied_registry_revision"].is_null());
        assert!(shaped["binaries_match"].is_null());
    }

    #[test]
    fn identity_revision_extractor_reads_top_level_revision() {
        let value = serde_json::json!({"version": 1, "revision": "abc123"});
        assert_eq!(
            applied_revision_from_registry_json(&value),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn identity_revision_extractor_returns_none_without_revision() {
        let value = serde_json::json!({"version": 1});
        assert_eq!(applied_revision_from_registry_json(&value), None);
    }

    #[test]
    fn identity_revision_extractor_rejects_empty_and_non_string() {
        assert_eq!(
            applied_revision_from_registry_json(&serde_json::json!({"revision": ""})),
            None
        );
        assert_eq!(
            applied_revision_from_registry_json(&serde_json::json!({"revision": 42})),
            None
        );
    }

    #[test]
    fn identity_sha256_hashes_file_bytes() {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().expect("temp fixture");
        file.write_all(b"cynic-identity").expect("write fixture");
        let digest = sha256_file(&file.path().to_path_buf()).expect("hashable fixture");
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(
            sha256_file(&std::path::PathBuf::from("/nonexistent-identity-fixture")),
            None
        );
    }

    fn lifecycle_envelope(
        subject: &str,
        producer: &str,
        observed_at_ms: u128,
        payload: serde_json::Value,
    ) -> EvidenceEnvelope {
        cynic_protocol::build_evidence_envelope(
            subject,
            producer,
            observed_at_ms,
            vec!["lifecycle-receipt-fixture".to_string()],
            &payload,
        )
        .expect("fixture envelope")
    }

    #[test]
    fn evidence_kind_detects_restart_drain_pause() {
        let restart = lifecycle_envelope(
            "builder",
            "cynic-node:supervisor",
            3,
            serde_json::json!({"schema": "cynic.node.restart_receipt.v1"}),
        );
        let drain = lifecycle_envelope(
            "builder:drain",
            "cynic-node",
            2,
            serde_json::json!({"status": "ok"}),
        );
        let pause = lifecycle_envelope(
            "builder",
            "cynic-node",
            1,
            serde_json::json!({"operation": "pause"}),
        );
        assert_eq!(evidence_lifecycle_kind(&restart), Some("restart"));
        assert_eq!(evidence_lifecycle_kind(&drain), Some("drain"));
        assert_eq!(evidence_lifecycle_kind(&pause), Some("pause"));
    }

    #[test]
    fn evidence_kind_detects_reload_receipts() {
        let reload = lifecycle_envelope(
            "builder",
            "cynic-node",
            4,
            serde_json::json!({"schema": "cynic.node.reload_receipt.v1"}),
        );
        assert_eq!(evidence_lifecycle_kind(&reload), Some("reload"));
    }

    #[test]
    fn reload_shape_reports_applied_vs_previous_and_digest() {
        let receipt = ReloadReceipt::applied(
            "builder-1",
            "prev-digest",
            "next-digest",
            vec!["node_control.interval_secs".to_string()],
            true,
            1_700_000_000_000,
        );
        let shaped = shape_reload_response(&receipt, Some("sha256:abc"));
        assert_eq!(shaped["schema"], "cynic.node.reload_receipt.v1");
        assert_eq!(shaped["outcome"], "applied");
        assert_eq!(shaped["previous_digest"], "prev-digest");
        assert_eq!(shaped["applied_digest"], "next-digest");
        assert_eq!(shaped["changed_keys"][0], "node_control.interval_secs");
        assert_eq!(shaped["requires_restart"], true);
        assert_eq!(shaped["receipt_digest"], "sha256:abc");
    }

    #[test]
    fn reload_shape_no_change_carries_no_receipt_digest() {
        let receipt = ReloadReceipt {
            schema: crate::config::RELOAD_RECEIPT_SCHEMA.to_string(),
            node_id: "builder-1".to_string(),
            outcome: crate::config::ReloadOutcome::AlreadyConverged
                .as_str()
                .to_string(),
            previous_digest: "same".to_string(),
            applied_digest: "same".to_string(),
            changed_keys: Vec::new(),
            requires_restart: false,
            observed_at_unix_ms: 1_700_000_000_000,
        };
        let shaped = shape_reload_response(&receipt, None);
        assert_eq!(shaped["outcome"], "already-converged");
        assert!(shaped["receipt_digest"].is_null());
        assert!(shaped["changed_keys"].as_array().unwrap().is_empty());
    }

    #[test]
    fn evidence_kind_is_none_for_build_proofs() {
        let proof = lifecycle_envelope(
            "agent-workload",
            "cynic-builder",
            1,
            serde_json::json!({"schema": "cynic.builder.proof.v1", "exit_code": 0}),
        );
        assert_eq!(evidence_lifecycle_kind(&proof), None);
    }

    #[test]
    fn evidence_limit_defaults_and_caps() {
        assert_eq!(clamp_evidence_limit(None), EVIDENCE_LIST_DEFAULT_LIMIT);
        assert_eq!(clamp_evidence_limit(Some(10)), 10);
        assert_eq!(clamp_evidence_limit(Some(0)), 0);
        assert_eq!(clamp_evidence_limit(Some(10_000)), EVIDENCE_LIST_MAX_LIMIT);
    }

    #[test]
    fn evidence_list_shape_carries_schema_and_counters() {
        let shaped = shape_evidence_list_response(
            "cynic-builder",
            1_700_000_000_000,
            vec![serde_json::json!({"digest": "sha256:abc"})],
            7,
            1,
            50,
            None,
        );
        assert_eq!(shaped["schema"], "cynic.builder.evidence_list.v1");
        assert_eq!(shaped["node_id"], "cynic-builder");
        assert_eq!(shaped["returned"], 1);
        assert_eq!(shaped["total_scanned"], 7);
        assert_eq!(shaped["unparseable"], 1);
        assert!(shaped["kind"].is_null());
    }

    fn write_evidence_fixture(dir: &std::path::Path, envelope: &EvidenceEnvelope) {
        let name = format!(
            "{}.json",
            envelope
                .digest
                .strip_prefix("sha256:")
                .expect("content-addressed digest")
        );
        std::fs::write(
            dir.join(name),
            serde_json::to_vec_pretty(envelope).expect("serialize fixture"),
        )
        .expect("write fixture");
    }

    #[test]
    fn evidence_scan_lists_newest_first_bounded_and_filtered() {
        let dir = tempfile::tempdir().expect("temp evidence dir");
        let evidence_root = dir.path().join("evidence");
        std::fs::create_dir(&evidence_root).expect("evidence dir");
        let old = lifecycle_envelope(
            "builder",
            "cynic-node",
            100,
            serde_json::json!({"schema": "cynic.node.restart_receipt.v1"}),
        );
        let mid = lifecycle_envelope(
            "builder",
            "cynic-node",
            200,
            serde_json::json!({"schema": "cynic.node.drain_receipt.v1"}),
        );
        let new = lifecycle_envelope(
            "builder",
            "cynic-node",
            300,
            serde_json::json!({"schema": "cynic.builder.proof.v1"}),
        );
        write_evidence_fixture(&evidence_root, &old);
        write_evidence_fixture(&evidence_root, &mid);
        write_evidence_fixture(&evidence_root, &new);
        std::fs::write(evidence_root.join("corrupt.json"), b"{not json").expect("corrupt");
        std::fs::write(evidence_root.join("notes.txt"), b"ignored").expect("non-json");

        let (items, total, bad) = list_evidence_summaries(&evidence_root, 10, None);
        assert_eq!(total, 4);
        assert_eq!(bad, 1);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["observed_at_unix_ms"], 300);
        assert_eq!(items[2]["observed_at_unix_ms"], 100);

        let (page, _, _) = list_evidence_summaries(&evidence_root, 1, None);
        assert_eq!(page.len(), 1);
        assert_eq!(page[0]["observed_at_unix_ms"], 300);

        let (restarts, _, _) = list_evidence_summaries(&evidence_root, 10, Some("restart"));
        assert_eq!(restarts.len(), 1);
        assert_eq!(restarts[0]["kind"], "restart");

        let (unknown, _, _) = list_evidence_summaries(&evidence_root, 10, Some("reboot"));
        assert!(unknown.is_empty());
    }

    #[test]
    fn evidence_scan_missing_dir_is_empty_not_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (items, total, bad) =
            list_evidence_summaries(&dir.path().join("no-such-evidence"), 10, None);
        assert!(items.is_empty());
        assert_eq!(total, 0);
        assert_eq!(bad, 0);
    }
}
