use crate::domain::events::KernelEvent;
use crate::domain::storage::StoragePort;
use crate::infra::task_health::TaskHealth;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub fn spawn_convergence_consumer(
    _storage: Arc<dyn StoragePort>,
    _event_tx: tokio::sync::broadcast::Sender<KernelEvent>,
    _task_health: Arc<TaskHealth>,
    _shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {})
}
