use std::fmt;
use std::sync::Arc;

use crate::domain::{coord::CoordPort, storage::StoragePort};
use crate::infra::config::StorageConfig;
use crate::storage::reconnectable::{ReconnectableCoord, ReconnectableStorage, StorageReconnector};

pub struct StorageRuntime {
    pub storage_port: Arc<dyn StoragePort>,
    pub coord: Arc<dyn CoordPort>,
    pub reconnector: Arc<StorageReconnector>,
}

impl fmt::Debug for StorageRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StorageRuntime")
            .field("storage_port", &"Arc<dyn StoragePort>")
            .field("coord", &"Arc<dyn CoordPort>")
            .field("reconnector", &self.reconnector)
            .finish()
    }
}

pub async fn connect_storage_runtime(storage_config: &StorageConfig) -> StorageRuntime {
    let (raw_storage, raw_coord): (Arc<dyn StoragePort>, Arc<dyn CoordPort>) =
        match crate::storage::SurrealHttpStorage::init(storage_config).await {
            Ok(storage) => {
                crate::klog!("[Ring 1] Storage: HEALTHY (SurrealDB HTTP)");
                let db = Arc::new(storage);
                (Arc::clone(&db) as _, Arc::clone(&db) as _)
            }
            Err(error) => {
                tracing::warn!(error = %error, "storage DEGRADED — will auto-reconnect when SurrealDB is available");
                (
                    Arc::new(crate::domain::storage::NullStorage) as _,
                    Arc::new(crate::domain::coord::UnavailableCoord::new(
                        error.to_string(),
                    )) as _,
                )
            }
        };

    let reconnectable_storage = Arc::new(ReconnectableStorage::new(raw_storage));
    let reconnectable_coord = Arc::new(ReconnectableCoord::new(raw_coord));
    let reconnector = Arc::new(StorageReconnector::new(
        reconnectable_storage.shared_lock(),
        reconnectable_coord.shared_lock(),
        storage_config.clone(),
    ));

    StorageRuntime {
        storage_port: reconnectable_storage as Arc<dyn StoragePort>,
        coord: reconnectable_coord as Arc<dyn CoordPort>,
        reconnector,
    }
}
