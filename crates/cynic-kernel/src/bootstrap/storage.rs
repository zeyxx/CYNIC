use std::fmt;
use std::sync::Arc;

use cynic_storage::book::CynicLedger;
use cynic_storage::projection::SqliteProjectionAdapter;
use cynic_storage::wal::WalFsAdapter;

use crate::domain::node_control::NodeControlStorePort;
use crate::domain::node_enrollment::NodeEnrollmentStorePort;
use crate::domain::{coord::CoordPort, storage::StoragePort};
use crate::infra::config::StorageConfig;

pub struct StorageRuntime {
    pub storage_port: Arc<dyn StoragePort>,
    pub coord: Arc<dyn CoordPort>,
    pub node_control: Option<Arc<dyn NodeControlStorePort>>,
    pub node_enrollment: Option<Arc<dyn NodeEnrollmentStorePort>>,
    pub ledger: Option<Arc<CynicLedger<WalFsAdapter>>>,
}

impl fmt::Debug for StorageRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StorageRuntime")
            .field("storage_port", &"Arc<dyn StoragePort>")
            .field("coord", &"Arc<dyn CoordPort>")
            .field("node_control_available", &self.node_control.is_some())
            .field("node_enrollment_available", &self.node_enrollment.is_some())
            .field("ledger_available", &self.ledger.is_some())
            .finish()
    }
}

pub async fn connect_storage_runtime(storage_config: &StorageConfig) -> StorageRuntime {
    // 1. Initialize Sovereign WAL + SQLite Book Ledger
    let ledger: Option<Arc<CynicLedger<WalFsAdapter>>> = {
        let wal_dir = std::path::Path::new(&storage_config.data_dir).join("wal");
        let proj_db = std::path::Path::new(&storage_config.data_dir).join("views.db");
        match WalFsAdapter::open(&wal_dir) {
            Ok(wal) => {
                let wal_arc = Arc::new(wal);
                match SqliteProjectionAdapter::open(&proj_db) {
                    Ok(proj) => match CynicLedger::open(wal_arc, proj) {
                        Ok(l) => {
                            crate::klog!(
                                "[Ring 1] CynicLedger (WAL + SQLite Projections): INITIALIZED"
                            );
                            Some(Arc::new(l))
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "CynicLedger open failed");
                            None
                        }
                    },
                    Err(e) => {
                        tracing::warn!(error = %e, "SqliteProjectionAdapter open failed");
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "WalFsAdapter open failed");
                None
            }
        }
    };

    let (storage_port, coord, node_control, node_enrollment): (
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
            tracing::warn!(error = %error, "storage DEGRADED — running with NullStorage");
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

    StorageRuntime {
        storage_port,
        coord,
        node_control,
        node_enrollment,
        ledger,
    }
}
