//! POST /mint-permit — Dogs-gated mint permit for PoIH attestation.
//!
//! Flow: wallet → Helius enrich (temporal) → behavioral_dog → permit if WAG/HOWL.
//! Does NOT use the full judge pipeline (no LLM Dogs, no CCM, no cache).
//! Cost: ~121 Helius credits (1 balance + 110 parsed history + 10 assets).

use axum::{extract::State, http::StatusCode, response::Json};
use std::sync::Arc;

use super::types::*;
use crate::domain::poih::{MINT_PERMIT_THRESHOLD, MintPermit, PERMIT_APPROVED_KINDS};
use crate::domain::wallet_enrichment::behavioral_dog;

/// Convert a VerdictKind to its lowercase string label.
fn kind_label(kind: &crate::domain::dog::VerdictKind) -> &'static str {
    match kind {
        crate::domain::dog::VerdictKind::Howl => "howl",
        crate::domain::dog::VerdictKind::Wag => "wag",
        crate::domain::dog::VerdictKind::Growl => "growl",
        crate::domain::dog::VerdictKind::Bark => "bark",
        crate::domain::dog::VerdictKind::Epoche => "epoche",
    }
}

pub async fn mint_permit_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MintPermitRequest>,
) -> Result<Json<MintPermitResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _ = &state; // AppState available for future rate-limiting / metrics wiring

    let wallet = req.wallet_address.trim().to_string();
    // Solana addresses: base58-encoded 32-byte public key → 32-44 chars
    if wallet.len() < 32 || wallet.len() > 44 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "wallet_address must be a valid Solana address (32-44 chars)".into(),
            }),
        ));
    }

    // Construct HeliusEnricher from env — same API key as the enricher in AppState.
    // Avoids touching AppState type (no new field required) while reusing the same key.
    let Some(enricher) = crate::backends::helius::HeliusEnricher::from_env() else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Helius enricher unavailable — HELIUS_API_KEY not configured".into(),
            }),
        ));
    };

    // Step 1: Enrich wallet with temporal behavioral data via Helius
    let profile = match enricher.enrich_wallet(&wallet).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            // No transaction history — wallet is either brand new or has no SWAP history.
            // Return EPOCHÉ: not enough data for a verdict (not a rejection).
            return Ok(Json(MintPermitResponse {
                permit: None,
                verdict_kind: "epoche".to_string(),
                q_score: 0.0,
                rejection_reason: Some(
                    "Wallet has no SWAP transaction history — insufficient data for behavioral evaluation".into(),
                ),
            }));
        }
        Err(e) => {
            tracing::warn!(wallet = %wallet, error = %e, "wallet enrichment failed for /mint-permit");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Wallet enrichment failed: {e}"),
                }),
            ));
        }
    };

    // Step 2: Score with behavioral deterministic dog (pure function, no I/O)
    let (kind, scores) = behavioral_dog(&profile);
    let kind_str = kind_label(&kind).to_string();

    // Q-score: arithmetic mean of all 6 axioms (mirrors behavioral_dog internals)
    let q_score_val = (scores.fidelity
        + scores.phi
        + scores.verify
        + scores.culture
        + scores.burn
        + scores.sovereignty)
        / 6.0;

    // Step 3: Gate on WAG/HOWL and minimum Q threshold
    let approved =
        PERMIT_APPROVED_KINDS.contains(&kind_str.as_str()) && q_score_val >= MINT_PERMIT_THRESHOLD;

    let permit = if approved {
        Some(MintPermit {
            wallet_address: wallet.clone(),
            proof_source: req.proof_source,
            q_score: q_score_val,
            verdict_kind: kind_str.clone(),
            approved: true,
            verdict_id: crate::infra::crypto::generate_secure_id(),
            // Integrity chain not yet wired (Task 7+). Empty string is honest.
            verdict_hash: String::new(),
            evaluated_at: chrono::Utc::now().to_rfc3339(),
        })
    } else {
        None
    };

    let rejection_reason = if !approved {
        Some(format!(
            "Wallet scored {kind_str} (Q={q_score_val:.3}). Minimum for permit: wag (Q≥{MINT_PERMIT_THRESHOLD:.3})",
        ))
    } else {
        None
    };

    Ok(Json(MintPermitResponse {
        permit,
        verdict_kind: kind_str,
        q_score: q_score_val,
        rejection_reason,
    }))
}
