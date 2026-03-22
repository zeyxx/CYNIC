//! SSE endpoint — real-time kernel event stream.
//!
//! `GET /events` streams KernelEvents as Server-Sent Events.
//! Zero new dependencies — axum + futures-util + tokio::sync::broadcast.
//! Browser-native: `new EventSource("/events")` just works.
//! Terminal: `curl -N http://host:3030/events` shows events as they arrive.

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::stream::Stream;
use std::convert::Infallible;
use std::sync::Arc;

use super::types::{AppState, KernelEvent};

/// GET /events — SSE stream of kernel events.
/// Public endpoint (no auth) — events are operational data.
/// Rate: ~1-10 events/minute under normal load. Keepalive every 15s.
pub async fn events_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.event_tx.subscribe();

    // Convert broadcast::Receiver into a Stream using unfold — no tokio-stream dep needed.
    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(kernel_event) => {
                    let event_type = event_type_name(&kernel_event);
                    let json = serde_json::to_string(&kernel_event).unwrap_or_default();
                    let sse_event = Event::default().event(event_type).data(json);
                    return Some((Ok(sse_event), rx));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::debug!(skipped = n, "SSE client lagged — skipping events");
                    continue; // Skip missed events, don't crash the stream
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    return None; // Channel closed — end stream
                }
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(15)))
}

fn event_type_name(event: &KernelEvent) -> &'static str {
    match event {
        KernelEvent::VerdictIssued { .. } => "verdict",
        KernelEvent::CrystalObserved { .. } => "crystal",
        KernelEvent::CrystalPromoted { .. } => "crystal_promoted",
        KernelEvent::DogFailed { .. } => "dog_failed",
        KernelEvent::SessionRegistered { .. } => "session",
        KernelEvent::BackfillComplete { .. } => "backfill",
        KernelEvent::Anomaly { .. } => "anomaly",
    }
}
