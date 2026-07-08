use crate::domain::events::KernelEvent;
use crate::domain::storage::StoragePort;
use crate::infra::task_health::TaskHealth;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

// WHY: Dormant logic kept for Phase 2, disabled as part of Option B deep purge
#[allow(dead_code)]
pub(crate) fn spawn_nightshift(
    _storage: Arc<dyn StoragePort>,
    _event_tx: tokio::sync::broadcast::Sender<KernelEvent>,
    _task_health: Arc<TaskHealth>,
    _shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {})
}
