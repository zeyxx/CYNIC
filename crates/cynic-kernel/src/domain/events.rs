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
    DogExpired {
        dog_id: String,
    },
    /// Discovery loop successfully re-registered a Dog that was missing.
    DogDiscovered {
        dog_id: String,
    },
    /// Contract delta changed — emitted each discovery cycle when expected ≠ actual.
    ContractDelta {
        missing: Vec<String>,
        expected: usize,
        fulfilled: bool,
    },
    /// Crystal shattered — instant dissolution due to catastrophic event.
    CrystalShattered {
        crystal_id: String,
        domain: String,
        reason: String,
    },
    StorageReconnected,
    /// A service announced itself via POST /services/register.
    /// Feeds the OS-level process table — absence of this event = unknown service.
    ServiceRegistered {
        service_id: String,
        tier: u8,
        node: String,
    },
    /// A service TTL expired or explicitly called DELETE /services/{id}.
    /// Kernel process table evicts the entry; organism can react.
    ServiceDown {
        service_id: String,
        reason: String, // "ttl_expired" | "explicit_unregister"
    },
}
