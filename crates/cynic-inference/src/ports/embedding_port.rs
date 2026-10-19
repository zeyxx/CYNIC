use async_trait::async_trait;

use crate::domain::embedding::{Embedding, EmbeddingError};

#[async_trait]
pub trait EmbeddingPort: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError>;

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// Whole-slice fanout with progress: backends that parallelize (cluster
    /// work-stealing) override this to keep ONE shared queue over all texts.
    /// Chunking here would hand each call a single sublot and deterministically
    /// starve every endpoint but the first-polled — never chunk before fanout.
    async fn embed_batch_fanout(
        &self,
        texts: &[&str],
        report: &(dyn Fn(usize, usize) + Sync),
    ) -> Result<Vec<Embedding>, EmbeddingError> {
        let out = self.embed_batch(texts).await?;
        report(out.len(), texts.len());
        Ok(out)
    }
}
