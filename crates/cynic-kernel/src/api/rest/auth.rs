//! Wallet-based authentication handlers.

use super::types::{AppState, ErrorResponse, Role};
use crate::infra::crypto;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Serialize)]
pub struct AuthInputResponse {
    pub nonce: String,
    pub statement: String,
    pub domain: String,
    pub timestamp: u64,
}

#[derive(Debug, Deserialize)]
pub struct AuthVerifyRequest {
    pub address: String,
    pub signature: String,
    pub nonce: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize)]
pub struct AuthVerifyResponse {
    pub role: String,
    pub expires_at: u64,
    #[serde(rename = "session_token")]
    pub session_id: String,
    pub address: String,
}

#[derive(Debug, Clone)]
pub(super) struct AuthSession {
    pub role: Role,
    // WHY: address is parsed from JWT but not currently used in auth middleware checks
    #[allow(dead_code)]
    pub address: String,
    pub expires_at: u64,
}

static AUTH_SESSIONS: OnceLock<RwLock<HashMap<String, AuthSession>>> = OnceLock::new();

pub(super) fn auth_sessions() -> &'static RwLock<HashMap<String, AuthSession>> {
    AUTH_SESSIONS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn build_auth_message(domain: &str, statement: &str, nonce: &str) -> Vec<u8> {
    format!("CYNIC AUTH\ndomain:{domain}\nnonce:{nonce}\nstatement:{statement}").into_bytes()
}

/// Generate a unique high-entropy nonce for wallet sign-in.
pub async fn auth_input_handler(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let nonce = crypto::generate_nonce();
    let timestamp = chrono::Utc::now().timestamp().max(0) as u64;

    let domain = std::env::var("CYNIC_DOMAIN").unwrap_or_else(|_| "localhost".to_string());

    Json(AuthInputResponse {
        nonce,
        statement: "Sign in to CYNIC Epistemic Immune System. This signature proves you own the sovereign identity associated with this wallet.".to_string(),
        domain,
        timestamp,
    })
}

/// Verify a wallet signature and issue a short-lived session token.
pub async fn auth_verify_handler(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<AuthVerifyRequest>,
) -> Response {
    let domain = std::env::var("CYNIC_DOMAIN").unwrap_or_else(|_| "localhost".to_string());
    let statement = "Sign in to CYNIC Epistemic Immune System. This signature proves you own the sovereign identity associated with this wallet.";
    let message = build_auth_message(&domain, statement, &payload.nonce);

    match crate::infra::crypto::verify_signature(
        &payload.address,
        &payload.signature,
        &payload.timestamp.to_string(),
        &message,
    ) {
        Ok(_) => {
            let session_id = crypto::generate_secure_id();
            let expires_at = (chrono::Utc::now().timestamp() + 12 * 3600) as u64;
            let session = AuthSession {
                role: Role::Cortex,
                address: payload.address.clone(),
                expires_at,
            };

            if let Ok(mut sessions) = auth_sessions().write() {
                sessions.insert(session_id.clone(), session);
            }

            Json(AuthVerifyResponse {
                role: Role::Cortex.to_string(),
                expires_at,
                session_id,
                address: payload.address,
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: format!("Signature verification failed: {e}"),
            }),
        )
            .into_response(),
    }
}
