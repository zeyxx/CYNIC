use std::sync::Arc;

mod peer;

use axum::{
    Extension, Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rand::random;
use serde::Serialize;

use super::local_peer::LocalPeerCredentials;
use super::types::AppState;
use cynic_protocol::{
    AGENT_SESSION_RECORD_SCHEMA, AGENT_SESSION_RENEWAL_REQUEST_SCHEMA, AgentRootLaunchPlan,
    AgentRootLaunchRequest, AgentSessionCredentialKind, AgentSessionRecord,
    AgentSessionRenewalPlan, AgentSessionRenewalRequest, AgentSessionState, AgentSessionTransport,
    compose_root_agent_launch, reconcile_agent_session_renewal,
};

const RESPONSE_SCHEMA: &str = "cynic.agent_root_launch_response.v1";

#[derive(Debug, Serialize)]
pub struct AgentRootLaunchResponse {
    schema: &'static str,
    status: String,
    plan: AgentRootLaunchPlan,
    session: Option<IssuedAgentSession>,
    #[serde(default)]
    unknowns: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct IssuedAgentSession {
    session_id: String,
    agent_id: String,
    node_id: String,
    authority_revision: u64,
    delegation_digest: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    generation: u64,
    credential_kind: AgentSessionCredentialKind,
    secret: bool,
}

pub async fn root_agent_launch_handler(
    State(state): State<Arc<AppState>>,
    peer: Option<Extension<LocalPeerCredentials>>,
    Json(request): Json<AgentRootLaunchRequest>,
) -> Response {
    let Some(runtime) = state.agent_sessions.as_ref() else {
        return unavailable(request.request_id, "agent_session_authority_missing");
    };
    let Some(Extension(peer)) = peer else {
        return unavailable(request.request_id, "local_peer_observation_missing");
    };
    if request.transport != AgentSessionTransport::LocalUnixPeer
        || request.credential_kind != AgentSessionCredentialKind::LocalPeerBinding
    {
        return unavailable(
            request.request_id,
            "local_root_launch_requires_peer_bound_credential",
        );
    }
    let now_unix_ms = now_unix_ms();
    let (transport_binding, peer_observation) = match peer::observe(runtime, &peer, now_unix_ms) {
        Ok(observation) => observation,
        Err(unknown) => return unavailable(request.request_id, unknown),
    };
    let plan =
        compose_root_agent_launch(&request, &runtime.authority, &peer_observation, now_unix_ms);
    if !plan.admitted() {
        let status = if plan.blockers.is_empty() {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::FORBIDDEN
        };
        return (
            status,
            Json(AgentRootLaunchResponse {
                schema: RESPONSE_SCHEMA,
                status: plan.status.clone(),
                plan,
                session: None,
                unknowns: Vec::new(),
            }),
        )
            .into_response();
    }

    let Some(session_request) = plan.session_request.as_ref() else {
        return unavailable(request.request_id, "admitted_session_request_missing");
    };
    let delegation_digest = session_request.delegation_digest.clone();
    let session_capabilities = session_request.capabilities.clone();
    let Some(expires_at_unix_ms) = now_unix_ms.checked_add(session_request.requested_ttl_ms) else {
        return unavailable(request.request_id, "agent_session_expiry_overflow");
    };
    let session_id = format!("local:{}", hex::encode(random::<[u8; 16]>()));
    let record = AgentSessionRecord {
        schema: AGENT_SESSION_RECORD_SCHEMA.to_string(),
        session_id: session_id.clone(),
        request_id: request.request_id,
        agent_id: request.agent_id.clone(),
        agent_type: Some(request.agent_type.clone()),
        node_id: runtime.node_id.clone(),
        delegation_digest: delegation_digest.clone(),
        authority_revision: runtime.authority.revision,
        credential_kind: AgentSessionCredentialKind::LocalPeerBinding,
        credential_digest: None,
        confirmation_key_sha256: None,
        transport_binding_sha256: Some(transport_binding),
        issued_at_unix_ms: now_unix_ms,
        expires_at_unix_ms,
        generation: 1,
        state: AgentSessionState::Active,
        revoked_at_unix_ms: None,
        revocation_reason: None,
        capabilities: session_capabilities,
        evidence_refs: peer_observation.evidence_refs,
    };
    if runtime.store.insert(record).await.is_err() {
        return unavailable(request.agent_id, "agent_session_store_insert_failed");
    }

    (
        StatusCode::CREATED,
        Json(AgentRootLaunchResponse {
            schema: RESPONSE_SCHEMA,
            status: "issued".to_string(),
            plan,
            session: Some(IssuedAgentSession {
                session_id,
                agent_id: request.agent_id,
                node_id: runtime.node_id.clone(),
                authority_revision: runtime.authority.revision,
                delegation_digest,
                issued_at_unix_ms: now_unix_ms,
                expires_at_unix_ms,
                generation: 1,
                credential_kind: AgentSessionCredentialKind::LocalPeerBinding,
                secret: false,
            }),
            unknowns: Vec::new(),
        }),
    )
        .into_response()
}

#[derive(Debug, Serialize)]
pub struct AgentSessionRenewalResponse {
    schema: &'static str,
    status: String,
    plan: AgentSessionRenewalPlan,
    session: Option<IssuedAgentSession>,
    #[serde(default)]
    unknowns: Vec<String>,
}

pub async fn renew_agent_session_handler(
    State(state): State<Arc<AppState>>,
    peer: Option<Extension<LocalPeerCredentials>>,
    Extension(record): Extension<AgentSessionRecord>,
    Json(request): Json<AgentSessionRenewalRequest>,
) -> Response {
    let Some(runtime) = state.agent_sessions.as_ref() else {
        return unavailable(request.request_id, "agent_session_authority_missing");
    };
    let Some(Extension(peer)) = peer else {
        return unavailable(request.request_id, "local_peer_observation_missing");
    };
    if request.schema != AGENT_SESSION_RENEWAL_REQUEST_SCHEMA {
        return unavailable(request.request_id, "agent_session_renewal_schema_mismatch");
    }
    let now_unix_ms = now_unix_ms();
    let (_, peer_observation) = match peer::observe(runtime, &peer, now_unix_ms) {
        Ok(observation) => observation,
        Err(unknown) => return unavailable(request.request_id, unknown),
    };
    let plan = reconcile_agent_session_renewal(
        &request,
        &runtime.authority,
        &record,
        &peer_observation,
        now_unix_ms,
    );
    if !plan.admitted() {
        let status = if plan.blockers.is_empty() {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::FORBIDDEN
        };
        return (
            status,
            Json(AgentSessionRenewalResponse {
                schema: "cynic.agent_session_renewal_response.v1",
                status: plan.status.clone(),
                plan,
                session: None,
                unknowns: Vec::new(),
            }),
        )
            .into_response();
    }

    let Some(expires_at_unix_ms) = plan.expires_at_unix_ms else {
        return unavailable(request.request_id, "admitted_renewal_expiry_missing");
    };
    let Some(generation) = plan.replacement_generation else {
        return unavailable(request.request_id, "admitted_renewal_generation_missing");
    };
    let session_id = format!("local:{}", hex::encode(random::<[u8; 16]>()));
    let mut evidence_refs = peer_observation.evidence_refs;
    evidence_refs.push(format!("agent-session-rotation:{now_unix_ms}"));
    let replacement = AgentSessionRecord {
        schema: AGENT_SESSION_RECORD_SCHEMA.to_string(),
        session_id: session_id.clone(),
        request_id: request.request_id.clone(),
        agent_id: record.agent_id.clone(),
        agent_type: record.agent_type.clone(),
        node_id: record.node_id.clone(),
        delegation_digest: record.delegation_digest.clone(),
        authority_revision: record.authority_revision,
        credential_kind: record.credential_kind,
        credential_digest: None,
        confirmation_key_sha256: record.confirmation_key_sha256.clone(),
        transport_binding_sha256: record.transport_binding_sha256.clone(),
        issued_at_unix_ms: now_unix_ms,
        expires_at_unix_ms,
        generation,
        state: AgentSessionState::Active,
        revoked_at_unix_ms: None,
        revocation_reason: None,
        capabilities: record.capabilities.clone(),
        evidence_refs,
    };
    let rotated = match runtime
        .store
        .rotate(
            &record.session_id,
            record.generation,
            replacement,
            now_unix_ms,
            &format!("agent-session-rotation:{now_unix_ms}"),
        )
        .await
    {
        Ok(rotated) => rotated,
        Err(_) => return unavailable(request.request_id, "agent_session_rotation_failed"),
    };

    (
        StatusCode::CREATED,
        Json(AgentSessionRenewalResponse {
            schema: "cynic.agent_session_renewal_response.v1",
            status: "rotated".to_string(),
            plan,
            session: Some(IssuedAgentSession {
                session_id: rotated.session_id,
                agent_id: rotated.agent_id,
                node_id: rotated.node_id,
                authority_revision: rotated.authority_revision,
                delegation_digest: rotated.delegation_digest,
                issued_at_unix_ms: rotated.issued_at_unix_ms,
                expires_at_unix_ms: rotated.expires_at_unix_ms,
                generation: rotated.generation,
                credential_kind: rotated.credential_kind,
                secret: false,
            }),
            unknowns: Vec::new(),
        }),
    )
        .into_response()
}

pub async fn revoke_agent_session_handler(
    State(state): State<Arc<AppState>>,
    Extension(record): Extension<AgentSessionRecord>,
) -> Response {
    let Some(runtime) = state.agent_sessions.as_ref() else {
        return unavailable(record.request_id, "agent_session_authority_missing");
    };
    let revoked_at_unix_ms = now_unix_ms();
    match runtime
        .store
        .revoke(
            &record.session_id,
            record.generation,
            revoked_at_unix_ms,
            "agent_process_shutdown",
            &format!("agent-session-self-revocation:{revoked_at_unix_ms}"),
        )
        .await
    {
        Ok(revoked) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "schema": "cynic.agent_session_revocation_report.v1",
                "status": "revoked",
                "session_id": revoked.session_id,
                "agent_id": revoked.agent_id,
                "generation": revoked.generation,
                "revoked_at_unix_ms": revoked.revoked_at_unix_ms,
                "secret": false,
            })),
        )
            .into_response(),
        Err(_) => unavailable(record.request_id, "agent_session_revocation_failed"),
    }
}

fn unavailable(request_id: String, unknown: &str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "schema": RESPONSE_SCHEMA,
            "status": "blocked_disconnected",
            "request_id": request_id,
            "unknowns": [unknown],
        })),
    )
        .into_response()
}

fn now_unix_ms() -> u64 {
    chrono::Utc::now().timestamp_millis().max(0) as u64
}
