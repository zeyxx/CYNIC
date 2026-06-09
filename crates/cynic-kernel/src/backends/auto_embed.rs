//! AutoRecoveryEmbedding — infra wrapper that hot-swaps embedding backends at runtime.
//! Lives in backends/ (infra layer) because it depends on tokio::sync::RwLock (K5).

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::domain::embedding::{Embedding, EmbeddingError, EmbeddingPort};

/// Wraps an EmbeddingPort that can be hot-swapped at runtime.
/// Starts with NullEmbedding, upgrades to real backend when discovered.
pub struct AutoRecoveryEmbedding {
    inner: RwLock<Arc<dyn EmbeddingPort>>,
}

impl std::fmt::Debug for AutoRecoveryEmbedding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoRecoveryEmbedding").finish()
    }
}

impl AutoRecoveryEmbedding {
    pub fn new(initial: Arc<dyn EmbeddingPort>) -> Self {
        Self {
            inner: RwLock::new(initial),
        }
    }

    /// Hot-swap the inner backend. Called by discovery loop.
    pub async fn upgrade(&self, backend: Arc<dyn EmbeddingPort>) {
        let mut guard = self.inner.write().await;
        *guard = backend;
    }
}

#[async_trait]
impl EmbeddingPort for AutoRecoveryEmbedding {
    async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let guard = self.inner.read().await;
        guard.embed(text).await
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        let guard = self.inner.read().await;
        guard.embed_batch(texts).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::embedding::{FixedEmbedding, NullEmbedding};

    #[tokio::test]
    async fn starts_null_then_upgrades() {
        let auto = AutoRecoveryEmbedding::new(Arc::new(NullEmbedding));
        assert!(auto.embed("test").await.is_err());

        let fixed = Arc::new(FixedEmbedding::new(vec![1.0, 0.0, 0.0]));
        auto.upgrade(fixed).await;

        let result = auto.embed("test").await.unwrap();
        assert_eq!(result.dimensions, 3);
    }

    #[tokio::test]
    async fn double_upgrade_uses_latest() {
        let auto = AutoRecoveryEmbedding::new(Arc::new(NullEmbedding));
        auto.upgrade(Arc::new(FixedEmbedding::new(vec![1.0, 0.0])))
            .await;
        auto.upgrade(Arc::new(FixedEmbedding::new(vec![1.0, 0.0, 0.0, 0.0])))
            .await;

        let result = auto.embed("test").await.unwrap();
        assert_eq!(result.dimensions, 4);
    }
}
