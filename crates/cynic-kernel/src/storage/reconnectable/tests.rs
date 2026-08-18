//! Unit tests for reconnectable storage and coord.

use std::sync::Arc;
use super::*;
use crate::domain::coord::{CoordPort, NullCoord};
use crate::domain::storage::{NullStorage, StoragePort};
use crate::infra::config::StorageConfig;

#[tokio::test]
async fn reconnectable_storage_delegates_ping_to_null() {
    let storage = ReconnectableStorage::new(Arc::new(NullStorage));
    assert!(
        storage.ping().await.is_err(),
        "NullStorage ping should fail"
    );
}

#[tokio::test]
async fn reconnectable_coord_delegates_to_null() {
    let coord = ReconnectableCoord::new(Arc::new(NullCoord));
    let result = coord.who(None).await;
    assert!(result.is_ok());
    let snap = result.unwrap();
    assert!(snap.agents.is_empty());
}

#[tokio::test]
async fn reconnector_is_degraded_with_null_storage() {
    let storage = ReconnectableStorage::new(Arc::new(NullStorage));
    let coord = ReconnectableCoord::new(Arc::new(NullCoord));
    let node_control = ReconnectableNodeControl::new(None);
    let node_enrollment = ReconnectableNodeEnrollment::new(None);
    let reconnector = StorageReconnector::new(
        storage.shared_lock(),
        coord.shared_lock(),
        node_control.shared_lock(),
        node_enrollment.shared_lock(),
        StorageConfig::default(),
    );
    assert!(reconnector.is_degraded().await);
}

#[tokio::test]
async fn reconnector_try_reconnect_fails_without_db() {
    let storage = ReconnectableStorage::new(Arc::new(NullStorage));
    let coord = ReconnectableCoord::new(Arc::new(NullCoord));
    let node_control = ReconnectableNodeControl::new(None);
    let node_enrollment = ReconnectableNodeEnrollment::new(None);
    let reconnector = StorageReconnector::new(
        storage.shared_lock(),
        coord.shared_lock(),
        node_control.shared_lock(),
        node_enrollment.shared_lock(),
        StorageConfig {
            url: "http://127.0.0.1:19999".to_string(), // unreachable
            namespace: "test".to_string(),
            database: "test".to_string(),
            ..StorageConfig::default()
        },
    );
    assert!(!reconnector.try_reconnect().await);
    assert!(storage.ping().await.is_err());
}
