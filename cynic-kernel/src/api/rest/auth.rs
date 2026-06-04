//! SIWS (Sign-In-With-Solana) authentication handlers.

use super::types::{AppState, ErrorResponse, Role};
use crate::infra::crypto;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct AuthInputResponse {
    pub nonce: String,
    pub statement: String,
    pub domain: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthVerifyRequest {
    pub address: String,
    pub signature: String,
    pub nonce: String,
}

#[derive(Debug, Serialize)]
pub struct AuthVerifyResponse {
    pub role: String,
    pub expires_at: u64,
}

/// Generate a unique high-entropy nonce for SIWS.
pub async fn auth_input_handler(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let nonce = crypto::generate_nonce();

    let domain = std::env::var("CYNIC_DOMAIN").unwrap_or_else(|_| "localhost".to_string());

    Json(AuthInputResponse {
        nonce,
        statement: "Sign in to CYNIC Epistemic Immune System. This signature proves you own the sovereign identity associated with this wallet.".to_string(),
        domain,
    })
}

/// Verify a SIWS signature.
pub async fn auth_verify_handler(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<AuthVerifyRequest>,
) -> Response {
    // 1. Verify the signature
    // The message format for SIWS is standard. For now, we verify the raw Ed25519 signature
    // against the nonce message.

    let message = format!(
        "Sign in to CYNIC Epistemic Immune System. This signature proves you own the sovereign identity associated with this wallet.\nNonce: {}",
        payload.nonce
    );

    match crate::infra::crypto::verify_signature(
        &payload.address,
        &payload.signature,
        &(chrono::Utc::now().timestamp().to_string()), // Simplified timestamp check
        message.as_bytes(),
    ) {
        Ok(_) => {
            // 2. Success - return Role and expiration
            // In Phase 2, we check if the wallet address is in the trusted admin list.
            Json(AuthVerifyResponse {
                role: Role::Cortex.to_string(),
                expires_at: (chrono::Utc::now().timestamp() + 3600) as u64,
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
