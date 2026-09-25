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
    /// A new agent task was pushed to the mempool.
    AgentTaskCreated {
        task_id: String,
        kind: String,
        domain: String,
    },
    /// An agent atomically claimed a pending task.
    AgentTaskClaimed {
        task_id: String,
        agent_id: String,
    },
    /// An agent task was completed or failed.
    AgentTaskCompleted {
        task_id: String,
    },
<<<<<<< HEAD
    /// A crash was detected on the host (GPU hang, softlockup, etc.).
    CrashDetected {
        kind: String,
        evidence_ref: String,
        severity: String,
    },
    /// A post-mortem report is ready for a crashed boot.
    PostMortemReady {
        report_id: String,
        boot_id: String,
    },
    /// A systemd unit entered a restart loop above threshold.
    ServiceLoopEscalation {
        unit: String,
        count: u64,
        window_secs: u64,
    },
=======
>>>>>>> github/main
}
