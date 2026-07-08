use crate::api::rest::types::AppState;
use crate::infra::task_health::TaskHealth;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

// WHY: Dormant logic kept for Phase 2, disabled as part of Option B deep purge
#[allow(dead_code)]
pub(crate) fn spawn_runtime_loops(
    _state: Arc<AppState>,
    _task_health: Arc<TaskHealth>,
    _shutdown: CancellationToken,
) -> Vec<tokio::task::JoinHandle<()>> {
    vec![]
}
