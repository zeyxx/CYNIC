//! In-memory model registry adapter.
//!
//! This adapter is intentionally inference-owned. It gives lab, conformance,
//! and runtime projections a bounded registry without making `cynic-kernel`
//! storage responsible for model backend ownership.

use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    domain::{
        backend_identity::{BackendCapability, BackendIdentity},
        model_registry::{ModelBackend, ModelCapability, ModelHealth},
    },
    ports::model_registry_port::{ModelRegistryError, ModelRegistryPort},
};

#[derive(Debug, Default)]
pub struct InMemoryModelRegistry {
    backends: RwLock<HashMap<String, ModelBackend>>,
}

impl InMemoryModelRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ModelRegistryPort for InMemoryModelRegistry {
    async fn upsert_model_backend(&self, backend: &ModelBackend) -> Result<(), ModelRegistryError> {
        if backend.backend_id.trim().is_empty() {
            return Err(ModelRegistryError::Rejected(
                "backend_id must not be empty".to_string(),
            ));
        }

        self.backends
            .write()
            .await
            .insert(backend.backend_id.clone(), backend.clone());
        Ok(())
    }

    async fn list_model_backends(
        &self,
        sovereign_only: bool,
    ) -> Result<Vec<ModelBackend>, ModelRegistryError> {
        let mut backends: Vec<ModelBackend> = self
            .backends
            .read()
            .await
            .values()
            .filter(|backend| !sovereign_only || backend.sovereign)
            .cloned()
            .collect();
        backends.sort_by(|left, right| left.backend_id.cmp(&right.backend_id));
        Ok(backends)
    }

    async fn update_model_health(
        &self,
        backend_id: &str,
        health: &ModelHealth,
        last_seen: i64,
    ) -> Result<(), ModelRegistryError> {
        let mut backends = self.backends.write().await;
        let Some(backend) = backends.get_mut(backend_id) else {
            return Err(ModelRegistryError::Rejected(format!(
                "unknown backend_id: {backend_id}"
            )));
        };
        backend.health = health.clone();
        backend.last_seen = last_seen;
        Ok(())
    }

    async fn prune_stale_backends(&self, older_than_ms: i64) -> Result<u64, ModelRegistryError> {
        let mut backends = self.backends.write().await;
        let before = backends.len();
        backends.retain(|_, backend| backend.last_seen >= older_than_ms);
        Ok((before - backends.len()) as u64)
    }

    async fn get_model_backend(
        &self,
        backend_id: &str,
    ) -> Result<Option<ModelBackend>, ModelRegistryError> {
        Ok(self.backends.read().await.get(backend_id).cloned())
    }

    async fn backend_identities(&self) -> Result<Vec<BackendIdentity>, String> {
        let mut identities: Vec<BackendIdentity> = self
            .backends
            .read()
            .await
            .values()
            .map(backend_identity)
            .collect();
        identities.sort_by(|left, right| left.backend_name.cmp(&right.backend_name));
        Ok(identities)
    }
}

fn backend_identity(backend: &ModelBackend) -> BackendIdentity {
    BackendIdentity {
        backend_name: backend.backend_id.clone(),
        display_name: format!("{}:{}", backend.engine, backend.model_id),
        model_name: Some(backend.model_id.clone()),
        sovereign: backend.sovereign,
        capabilities: backend
            .capabilities
            .iter()
            .filter_map(backend_capability)
            .collect(),
    }
}

fn backend_capability(capability: &ModelCapability) -> Option<BackendCapability> {
    match capability {
        ModelCapability::Chat => Some(BackendCapability::Chat),
        ModelCapability::Embedding => Some(BackendCapability::Embedding),
        ModelCapability::ToolUse => Some(BackendCapability::ToolUse),
        ModelCapability::JsonMode => Some(BackendCapability::JsonMode),
        ModelCapability::Summarization => Some(BackendCapability::Summarization),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::model_registry::InferenceEngine;

    #[tokio::test]
    async fn stores_filters_updates_and_prunes_model_backends() {
        let registry = InMemoryModelRegistry::new();
        let sovereign = backend("sovereign", true, 100);
        let cloud = backend("cloud", false, 50);

        registry.upsert_model_backend(&cloud).await.unwrap();
        registry.upsert_model_backend(&sovereign).await.unwrap();

        let sovereign_backends = registry.list_model_backends(true).await.unwrap();
        assert_eq!(sovereign_backends.len(), 1);
        assert_eq!(sovereign_backends[0].backend_id, "sovereign");

        registry
            .update_model_health("sovereign", &ModelHealth::Healthy, 150)
            .await
            .unwrap();
        let updated = registry
            .get_model_backend("sovereign")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.health, ModelHealth::Healthy);
        assert_eq!(updated.last_seen, 150);

        assert_eq!(registry.prune_stale_backends(100).await.unwrap(), 1);
        assert!(registry.get_model_backend("cloud").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn exposes_backend_identities_from_inference_registry() {
        let registry = InMemoryModelRegistry::new();
        registry
            .upsert_model_backend(&backend("local-llama", true, 100))
            .await
            .unwrap();

        let identities = registry.backend_identities().await.unwrap();
        assert_eq!(identities.len(), 1);
        assert_eq!(identities[0].backend_name, "local-llama");
        assert!(identities[0].sovereign);
        assert_eq!(
            identities[0].capabilities,
            vec![BackendCapability::Chat, BackendCapability::JsonMode]
        );
    }

    fn backend(backend_id: &str, sovereign: bool, last_seen: i64) -> ModelBackend {
        ModelBackend {
            backend_id: backend_id.to_string(),
            engine: InferenceEngine::LlamaServer,
            endpoint: "http://127.0.0.1:8080".to_string(),
            model_id: "qwen.gguf".to_string(),
            node_id: "node-a".to_string(),
            sovereign,
            capabilities: vec![ModelCapability::Chat, ModelCapability::JsonMode],
            discovered_at: 1,
            last_seen,
            health: ModelHealth::Unknown,
        }
    }
}
