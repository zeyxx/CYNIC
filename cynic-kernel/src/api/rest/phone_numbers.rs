//! CallShield REST endpoints — phone-number domain read API.
//!
//! GET /phone-numbers/blocklist   — top-N numbers by lowest sovereignty (worst spam first).
//! GET /phone-numbers/reporter-stats — device reporter agreement rate + tier (MVP placeholder).

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

use super::response::storage_error;
use super::types::{
    AppState, BlocklistEntry, BlocklistResponse, ErrorResponse, ReporterStatsResponse,
};

// ── QUERY PARAMS ───────────────────────────────────────────

/// Query parameters for GET /phone-numbers/blocklist.
#[derive(Debug, Deserialize)]
pub struct BlocklistQuery {
    /// Maximum entries to return. Defaults to 1000, capped at 50 000.
    pub n: Option<u32>,
}

// ── HANDLERS ───────────────────────────────────────────────

/// GET /phone-numbers/blocklist?n=10000
///
/// Returns the top-N phone numbers with the lowest sovereignty score (worst spam first).
/// The mobile app downloads this on first install to populate its local cache.
pub async fn blocklist_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<BlocklistQuery>,
) -> Result<Json<BlocklistResponse>, (StatusCode, Json<ErrorResponse>)> {
    const DEFAULT_N: u32 = 1_000;
    const MAX_N: u32 = 50_000;

    let n = params.n.unwrap_or(DEFAULT_N).min(MAX_N);

    let verdicts = state
        .storage
        .list_verdicts_by_domain("phone-number", n)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "blocklist storage query failed");
            storage_error()
        })?;

    let numbers: Vec<BlocklistEntry> = verdicts
        .into_iter()
        .map(|v| BlocklistEntry {
            number: v.stimulus_summary.clone(),
            sovereignty: v.q_score.sovereignty,
            q_score: v.q_score.total,
            verdict: format!("{:?}", v.kind),
        })
        .collect();

    let count = numbers.len();
    let generated_at = chrono::Utc::now().to_rfc3339();

    Ok(Json(BlocklistResponse {
        numbers,
        count,
        generated_at,
    }))
}

/// GET /phone-numbers/reporter-stats
///
/// Returns agreement rate and tier for the calling device.
/// MVP: returns placeholder stats — real per-device tracking is Plan 3.
pub async fn reporter_stats_handler(
    State(_state): State<Arc<AppState>>,
) -> Json<ReporterStatsResponse> {
    // MVP placeholder — per-device tracking requires a device identity header
    // and a reporter_stats table (Plan 3). Tier is "anonymous" until that lands.
    Json(ReporterStatsResponse {
        agreement_rate: None,
        reports_total: 0,
        tier: "anonymous".to_string(),
    })
}
