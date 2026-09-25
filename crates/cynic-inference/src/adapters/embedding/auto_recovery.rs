//! AutoRecoveryEmbedding — hot-swaps embedding backends at runtime.

use std::sync::Arc;

use async_trait::async_trait;
use cynic_inference::embedding::{Embedding, EmbeddingError, EmbeddingPort};
use tokio::sync::RwLock;

<<<<<<<< HEAD:crates/cynic-inference/src/adapters/embedding/auto_recovery.rs
use crate::domain::embedding::{Embedding, EmbeddingError};
use crate::ports::embedding_port::EmbeddingPort;

========
/// Wraps an EmbeddingPort that can be hot-swapped at runtime.
/// Starts with NullEmbedding, upgrades to real backend when discovered.
>>>>>>>> github/main:crates/cynic-kernel/src/backends/auto_embed.rs
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
<<<<<<<< HEAD:crates/cynic-inference/src/adapters/embedding/auto_recovery.rs
    use crate::adapters::embedding::null::{FixedEmbedding, NullEmbedding};
========
    use cynic_inference::embedding::{FixedEmbedding, NullEmbedding};
>>>>>>>> github/main:crates/cynic-kernel/src/backends/auto_embed.rs

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
