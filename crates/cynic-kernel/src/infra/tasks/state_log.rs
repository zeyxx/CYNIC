
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use crate::api::rest::types::AppState;
use crate::infra::task_health::TaskHealth;

pub fn spawn_state_log(
    _state: Arc<AppState>,
    _task_health: Arc<TaskHealth>,
    _shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move { })
}
