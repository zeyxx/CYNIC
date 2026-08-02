use std::sync::Arc;

use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};

use super::super::{
    local_peer::LocalPeerCredentials,
    types::{
        AppState, AuthenticatedPrincipal, AuthenticationMechanism, ErrorResponse, Role,
        authorize_agent_session_request,
    },
};
use crate::domain::agent_session::local_transport_binding_sha256;

/// Resolves a presented local agent-session reference against durable state and
/// the kernel-observed Unix peer. `Ok((request, false))` means no agent session
/// was presented and lets the caller continue with the other auth mechanisms.
pub(super) async fn authenticate(
    state: &Arc<AppState>,
    request: Request,
    method: &str,
    path: &str,
) -> Result<(Request, bool), Response> {
    let Some(session_id) = request
        .headers()
        .get("X-Cynic-Agent-Session")
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string)
    else {
        return Ok((request, false));
    };
    let Some(runtime) = state.agent_sessions.as_ref() else {
        return Err(authentication_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Agent-session authority is unavailable",
        ));
    };
    let Some(peer) = request.extensions().get::<LocalPeerCredentials>() else {
        return Err(authentication_error(
            StatusCode::UNAUTHORIZED,
            "Agent sessions require an observed local Unix peer",
        ));
    };
    let binding = local_transport_binding_sha256(
        &runtime.node_id,
        &runtime.boot_id,
        peer.process_id,
        peer.process_start_time_ticks,
        peer.user_id,
        peer.group_id,
    )
    .map_err(|_| {
        authentication_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Local peer binding could not be observed",
        )
    })?;
    let now_unix_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
    let record = runtime
        .store
        .resolve_active(&session_id, now_unix_ms)
        .await
        .map_err(|_| {
            authentication_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Agent-session read-back is unavailable",
            )
        })?
        .ok_or_else(|| {
            authentication_error(
                StatusCode::UNAUTHORIZED,
                "Agent session is absent, expired, or revoked",
            )
        })?;
    if record.node_id != runtime.node_id
        || record.transport_binding_sha256.as_deref() != Some(binding.as_str())
    {
        return Err(authentication_error(
            StatusCode::UNAUTHORIZED,
            "Agent session does not match the observed local peer",
        ));
    }
    if path != "/auth/agent/session/revoke"
        && path != "/auth/agent/session/renew"
        && !authorize_agent_session_request(&record, method, path)
    {
        return Err(authentication_error(
            StatusCode::FORBIDDEN,
            "Agent session capability does not authorize this request",
        ));
    }

    let mut request = request;
    request.extensions_mut().insert(Role::Internal);
    request.extensions_mut().insert(AuthenticatedPrincipal {
        subject: Some(record.agent_id.clone()),
        role: Role::Internal,
        mechanism: AuthenticationMechanism::LocalAgentSession,
    });
    request.extensions_mut().insert(record);
    Ok((request, true))
}

fn authentication_error(status: StatusCode, error: &str) -> Response {
    (
        status,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
        .into_response()
}
