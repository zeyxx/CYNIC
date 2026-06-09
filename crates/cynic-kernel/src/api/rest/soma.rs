//! REST API handler for Soma — slot semaphore status query.

use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};

/// Response payload — slot availability status per Dog.
#[derive(Debug, Clone, Serialize)]
pub struct SomaStatusResponse {
    /// dog_id → slot info
    pub dogs: Vec<DogSlotStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DogSlotStatus {
    pub dog_id: String,
    pub total_slots: u32,
    pub available_slots: u32,
}

/// GET /soma/status — Query slot semaphore availability across all Dogs.
///
/// Response (200): { "dogs": [ { "dog_id": "...", "total_slots": 4, "available_slots": 3 }, ... ] }
pub async fn soma_request_handler(
    State(state): State<Arc<AppState>>,
    // NOTE: body is ignored — this endpoint is now a status query, not a gate request.
    // The SlotSemaphore system inside Judge::evaluate handles all slot coordination.
    _body: axum::body::Body,
) -> Result<(StatusCode, Json<SomaStatusResponse>), (StatusCode, Json<ErrorResponse>)> {
    let judge = state.judge.load_full();
    let dog_ids = judge.dog_ids();

    let dogs: Vec<DogSlotStatus> = dog_ids
        .iter()
        .filter_map(|dog_id| {
            state.slot_semaphores.get(dog_id).map(|sem| DogSlotStatus {
                dog_id: dog_id.clone(),
                total_slots: sem.total_slots(),
                available_slots: sem.available(),
            })
        })
        .collect();

    Ok((StatusCode::OK, Json(SomaStatusResponse { dogs })))
}
