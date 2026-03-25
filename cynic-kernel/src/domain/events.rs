//! Kernel events — domain-level event types consumed by all delivery layers.
//!
//! Lives in `domain/` so both REST and MCP can import without cross-layer leakage.
//! Rule #32: application services never import from delivery layers.

/// Events emitted by the kernel — consumed by SSE, MCP, WebSocket, and dashboards.
/// Clone-cheap: all payloads are small strings/numbers.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type")]
pub enum KernelEvent {
    VerdictIssued {
        verdict_id: String,
        domain: String,
        verdict: String,
        q_score: f64,
    },
    CrystalObserved {
        crystal_id: String,
        domain: String,
    },
    DogFailed {
        dog_id: String,
        error: String,
    },
    SessionRegistered {
        agent_id: String,
    },
    BackfillComplete {
        count: u32,
    },
    Anomaly {
        kind: String,
        message: String,
        severity: String,
    },
}
