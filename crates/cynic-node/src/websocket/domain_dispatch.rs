use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::Mutex;

use super::PendingVerdict;

/// Native federation mutation remains disabled until the node receives an
/// injected short-lived workload signer and can bind the exact stimulus,
/// verdict, timestamp, and node identity into completion evidence.
pub(super) fn spawn_native_handler(
    domain: &str,
    _stimulus_id: String,
    _content: serde_json::Value,
    _verdicts_queue: Arc<Mutex<VecDeque<PendingVerdict>>>,
) -> bool {
    tracing::warn!(
        domain,
        blocker = "runtime_federation_workload_signer_unavailable",
        "native federation mutation refused"
    );
    false
}
