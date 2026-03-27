//! SSE endpoint — real-time kernel event stream.
//!
//! `GET /events` streams KernelEvents as Server-Sent Events.
//! Zero new dependencies — axum + futures-util + tokio::sync::broadcast.
//! Browser-native: `new EventSource("/events")` just works.
//! Terminal: `curl -N http://host:3030/events` shows events as they arrive.

use axum::{
    extract::State,
    http::StatusCode,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use std::convert::Infallible;
use std::sync::Arc;

use super::types::AppState;
use crate::domain::events::KernelEvent;

/// GET /events — SSE stream of kernel events.
/// Public endpoint (no auth) — events are operational data.
/// F23: Limited to 32 concurrent connections (sse_semaphore) to prevent FD exhaustion.
/// Rate: ~1-10 events/minute under normal load. Keepalive every 15s.
pub async fn events_handler(State(state): State<Arc<AppState>>) -> Response {
    // F23: Acquire SSE permit — 503 if all slots taken.
    let Ok(permit) = state.sse_semaphore.clone().try_acquire_owned() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "SSE connection limit reached",
        )
            .into_response();
    };

    let rx = state.event_tx.subscribe();

    // Convert broadcast::Receiver into a Stream using unfold — no tokio-stream dep needed.
    // The permit is moved into the unfold state and held for the stream's lifetime.
    let stream = futures_util::stream::unfold((rx, Some(permit)), |(mut rx, permit)| async move {
        loop {
            match rx.recv().await {
                Ok(kernel_event) => {
                    let event_type = event_type_name(&kernel_event);
                    match serde_json::to_string(&kernel_event) {
                        Ok(json) => {
                            let sse_event = Event::default().event(event_type).data(json);
                            return Some((Ok::<_, Infallible>(sse_event), (rx, permit)));
                        }
                        Err(e) => {
                            tracing::warn!(event_type, error = %e, "SSE event serialization failed — skipping");
                            continue;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::debug!(skipped = n, "SSE client lagged — skipping events");
                    continue; // Skip missed events, don't crash the stream
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    return None; // Channel closed — end stream (permit dropped → slot freed)
                }
            }
        }
    });

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(15)))
        .into_response()
}

fn event_type_name(event: &KernelEvent) -> &'static str {
    match event {
        KernelEvent::VerdictIssued { .. } => "verdict",
        KernelEvent::CrystalObserved { .. } => "crystal",
        KernelEvent::DogFailed { .. } => "dog_failed",
        KernelEvent::SessionRegistered { .. } => "session",
        KernelEvent::BackfillComplete { .. } => "backfill",
        KernelEvent::Anomaly { .. } => "anomaly",
    }
}
