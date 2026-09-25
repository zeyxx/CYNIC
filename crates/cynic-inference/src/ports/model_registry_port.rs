use async_trait::async_trait;

use crate::domain::backend_identity::{BackendHealthSnapshot, BackendIdentity};
<<<<<<< HEAD
use crate::infra::model_registry::{ModelBackend, ModelHealth};
=======
use crate::domain::model_registry::{ModelBackend, ModelHealth};
>>>>>>> github/main

#[derive(Debug, Clone, thiserror::Error)]
pub enum ModelRegistryError {
    #[error("model registry unavailable: {0}")]
    Unavailable(String),
    #[error("model registry rejected operation: {0}")]
    Rejected(String),
}

#[async_trait]
pub trait ModelRegistryPort: Send + Sync {
    async fn upsert_model_backend(&self, backend: &ModelBackend) -> Result<(), ModelRegistryError>;
    async fn list_model_backends(
        &self,
        sovereign_only: bool,
    ) -> Result<Vec<ModelBackend>, ModelRegistryError>;
    async fn update_model_health(
        &self,
        backend_id: &str,
        health: &ModelHealth,
        last_seen: i64,
    ) -> Result<(), ModelRegistryError>;
    async fn prune_stale_backends(&self, older_than_ms: i64) -> Result<u64, ModelRegistryError>;
    async fn get_model_backend(
        &self,
        backend_id: &str,
    ) -> Result<Option<ModelBackend>, ModelRegistryError>;

    async fn backend_identities(&self) -> Result<Vec<BackendIdentity>, String>;
}

#[async_trait]
pub trait BackendCatalogPort: Send + Sync {
    async fn backend_health(&self) -> Result<Vec<BackendHealthSnapshot>, String>;
}
