use std::fmt;
use std::sync::Arc;

use crate::domain::node_control::NodeControlStorePort;
use crate::domain::node_enrollment::NodeEnrollmentStorePort;
use crate::domain::{coord::CoordPort, storage::StoragePort};
use crate::infra::config::StorageConfig;
use crate::storage::reconnectable::{
    ReconnectableCoord, ReconnectableNodeControl, ReconnectableNodeEnrollment,
    ReconnectableStorage, StorageReconnector,
};

pub struct StorageRuntime {
    pub storage_port: Arc<dyn StoragePort>,
    pub coord: Arc<dyn CoordPort>,
    pub reconnector: Arc<StorageReconnector>,
    pub node_control: Option<Arc<dyn NodeControlStorePort>>,
    pub node_enrollment: Option<Arc<dyn NodeEnrollmentStorePort>>,
}

impl fmt::Debug for StorageRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StorageRuntime")
            .field("storage_port", &"Arc<dyn StoragePort>")
            .field("coord", &"Arc<dyn CoordPort>")
            .field("reconnector", &self.reconnector)
            .field("node_control_available", &self.node_control.is_some())
            .field("node_enrollment_available", &self.node_enrollment.is_some())
            .finish()
    }
}

pub async fn connect_storage_runtime(storage_config: &StorageConfig) -> StorageRuntime {
    let (raw_storage, raw_coord, node_control, node_enrollment): (
        Arc<dyn StoragePort>,
        Arc<dyn CoordPort>,
        Option<Arc<dyn NodeControlStorePort>>,
        Option<Arc<dyn NodeEnrollmentStorePort>>,
    ) = match crate::storage::SurrealHttpStorage::init(storage_config).await {
        Ok(storage) => {
            crate::klog!("[Ring 1] Storage: HEALTHY (SurrealDB HTTP)");
            let db = Arc::new(storage);
            (
                Arc::clone(&db) as _,
                Arc::clone(&db) as _,
                Some(Arc::clone(&db) as _),
                Some(Arc::clone(&db) as _),
            )
        }
        Err(error) => {
            tracing::warn!(error = %error, "storage DEGRADED — will auto-reconnect when SurrealDB is available — proof bundle will carry explicit blocked (single_endpoint_no_replication + storage_degraded), audit loop continues without panic");
            (
                Arc::new(crate::domain::storage::NullStorage) as _,
                Arc::new(crate::domain::coord::UnavailableCoord::new(
                    error.to_string(),
                )) as _,
                None,
                None,
            )
        }
    };

    let reconnectable_storage = Arc::new(ReconnectableStorage::new(raw_storage));
    let reconnectable_coord = Arc::new(ReconnectableCoord::new(raw_coord));
    let reconnectable_node_control = Arc::new(ReconnectableNodeControl::new(node_control));
    let reconnectable_node_enrollment = Arc::new(ReconnectableNodeEnrollment::new(node_enrollment));
    let reconnector = Arc::new(StorageReconnector::new(
        reconnectable_storage.shared_lock(),
        reconnectable_coord.shared_lock(),
        reconnectable_node_control.shared_lock(),
        reconnectable_node_enrollment.shared_lock(),
        storage_config.clone(),
    ));

    StorageRuntime {
        storage_port: reconnectable_storage as Arc<dyn StoragePort>,
        coord: reconnectable_coord as Arc<dyn CoordPort>,
        reconnector,
        node_control: Some(reconnectable_node_control as Arc<dyn NodeControlStorePort>),
        node_enrollment: Some(reconnectable_node_enrollment as Arc<dyn NodeEnrollmentStorePort>),
    }
}
