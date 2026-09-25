//! REST handlers for temporal epoch clock — /temporal/epoch.
//!
//! Provides read and bounded mutation of the kernel's logical epoch clock.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::api::rest::types::AppState;

/// GET /temporal/epoch — returns the current epoch observation.
pub(crate) async fn epoch_get_handler(
    State(state): State<std::sync::Arc<AppState>>,
) -> impl IntoResponse {
    let observation = state.temporal_clock.observe();
    let response = serde_json::json!({
        "schema": "cynic.temporal.epoch.v1",
        "epoch": observation.current_epoch.id(),
        "last_mark_timestamp_unix": observation.last_mark_timestamp_unix,
    });
    (StatusCode::OK, Json(response))
}

/// POST /temporal/epoch/advance — advance the epoch clock by one.
/// Bounded mutation: requires kernel-local auth or operator token.
pub(crate) async fn epoch_advance_handler(
    State(state): State<std::sync::Arc<AppState>>,
) -> impl IntoResponse {
    let new_epoch = state.temporal_clock.advance();
    let observation = state.temporal_clock.observe();
    let response = serde_json::json!({
        "schema": "cynic.temporal.epoch_advance.v1",
        "action": "advance",
        "new_epoch": new_epoch.id(),
        "last_mark_timestamp_unix": observation.last_mark_timestamp_unix,
    });
    (StatusCode::OK, Json(response))
}
