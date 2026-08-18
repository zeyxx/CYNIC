//! Reconnection lifecycle engine for storage and coordination ports.

use std::sync::{Arc, RwLock};

use crate::domain::coord::CoordPort;
use crate::domain::node_control::NodeControlStorePort;
use crate::domain::node_enrollment::NodeEnrollmentStorePort;
use crate::domain::storage::StoragePort;
use crate::infra::config::StorageConfig;

/// Holds shared locks to both storage + coord, and the config needed to
/// reconnect. A background task calls try_reconnect() periodically.
pub struct StorageReconnector {
    storage_lock: Arc<RwLock<Arc<dyn StoragePort>>>,
    coord_lock: Arc<RwLock<Arc<dyn CoordPort>>>,
    node_control_lock: Arc<RwLock<Option<Arc<dyn NodeControlStorePort>>>>,
    node_enrollment_lock: Arc<RwLock<Option<Arc<dyn NodeEnrollmentStorePort>>>>,
    config: StorageConfig,
}

impl std::fmt::Debug for StorageReconnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageReconnector")
            .field("url", &self.config.url)
            .finish()
    }
}

impl StorageReconnector {
    pub fn new(
        storage_lock: Arc<RwLock<Arc<dyn StoragePort>>>,
        coord_lock: Arc<RwLock<Arc<dyn CoordPort>>>,
        node_control_lock: Arc<RwLock<Option<Arc<dyn NodeControlStorePort>>>>,
        node_enrollment_lock: Arc<RwLock<Option<Arc<dyn NodeEnrollmentStorePort>>>>,
        config: StorageConfig,
    ) -> Self {
        Self {
            storage_lock,
            coord_lock,
            node_control_lock,
            node_enrollment_lock,
            config,
        }
    }

    /// Check if current storage is degraded (NullStorage or unreachable).
    pub async fn is_degraded(&self) -> bool {
        let current = self
            .storage_lock
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        current.ping().await.is_err()
    }

    /// Attempt to reconnect to SurrealDB. Returns true on success.
    pub async fn try_reconnect(&self) -> bool {
        if !self.is_degraded().await {
            return false;
        }

        match crate::storage::SurrealHttpStorage::init(&self.config).await {
            Ok(db) => {
                let db = Arc::new(db);

                // Swap storage
                match self.storage_lock.write() {
                    Ok(mut guard) => *guard = Arc::clone(&db) as Arc<dyn StoragePort>,
                    Err(e) => {
                        tracing::error!(error = %e, "storage RwLock poisoned during reconnect");
                        return false;
                    }
                }

                // Swap coord
                match self.coord_lock.write() {
                    Ok(mut guard) => *guard = Arc::clone(&db) as Arc<dyn CoordPort>,
                    Err(e) => {
                        tracing::error!(error = %e, "coord RwLock poisoned during reconnect");
                    }
                }
                if let Ok(mut guard) = self.node_control_lock.write() {
                    *guard = Some(Arc::clone(&db) as Arc<dyn NodeControlStorePort>);
                }
                if let Ok(mut guard) = self.node_enrollment_lock.write() {
                    *guard = Some(Arc::clone(&db) as Arc<dyn NodeEnrollmentStorePort>);
                }

                tracing::info!(
                    url = %self.config.url,
                    ns = %self.config.namespace,
                    db = %self.config.database,
                    "storage RECONNECTED — NullStorage → SurrealDB"
                );
                true
            }
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    url = %self.config.url,
                    "storage reconnect failed — will retry"
                );
                false
            }
        }
    }
}
