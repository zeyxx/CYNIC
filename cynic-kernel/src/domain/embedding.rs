//! EmbeddingPort — contract for generating vector embeddings from text.
//! Used by CCM (semantic clustering), semantic verdict cache, and multi-source ingestion.
//! Pure domain — zero external dependencies.

use async_trait::async_trait;

/// A vector embedding with metadata.
#[derive(Debug, Clone)]
pub struct Embedding {
    pub vector: Vec<f32>,
    pub dimensions: usize,
    pub prompt_tokens: u32,
}

impl Embedding {
    /// Cosine similarity between two embeddings. Returns -1.0 to 1.0.
    pub fn cosine_similarity(&self, other: &Embedding) -> f64 {
        if self.vector.len() != other.vector.len() {
            return 0.0;
        }
        let dot: f64 = self.vector.iter().zip(&other.vector)
            .map(|(a, b)| *a as f64 * *b as f64)
            .sum();
        let norm_a: f64 = self.vector.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
        let norm_b: f64 = other.vector.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Embedding unreachable: {0}")]
    Unreachable(String),
    #[error("Embedding timed out after {ms}ms")]
    Timeout { ms: u64 },
    #[error("Embedding protocol error: {0}")]
    Protocol(String),
}

/// Port for generating embeddings. Adapter implementations call external servers.
#[async_trait]
pub trait EmbeddingPort: Send + Sync {
    /// Embed a single text. Returns a normalized vector.
    async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError>;

    /// Embed multiple texts in a batch. Default: sequential calls.
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }
}

/// Null implementation — graceful degradation when no embedding server is available.
pub struct NullEmbedding;

#[async_trait]
impl EmbeddingPort for NullEmbedding {
    async fn embed(&self, _text: &str) -> Result<Embedding, EmbeddingError> {
        Err(EmbeddingError::Unreachable("no embedding server configured".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_vectors() {
        let a = Embedding { vector: vec![1.0, 0.0, 0.0], dimensions: 3, prompt_tokens: 0 };
        let b = Embedding { vector: vec![1.0, 0.0, 0.0], dimensions: 3, prompt_tokens: 0 };
        assert!((a.cosine_similarity(&b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = Embedding { vector: vec![1.0, 0.0], dimensions: 2, prompt_tokens: 0 };
        let b = Embedding { vector: vec![0.0, 1.0], dimensions: 2, prompt_tokens: 0 };
        assert!(a.cosine_similarity(&b).abs() < 1e-6);
    }

    #[test]
    fn cosine_opposite_vectors() {
        let a = Embedding { vector: vec![1.0, 0.0], dimensions: 2, prompt_tokens: 0 };
        let b = Embedding { vector: vec![-1.0, 0.0], dimensions: 2, prompt_tokens: 0 };
        assert!((a.cosine_similarity(&b) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_different_lengths_returns_zero() {
        let a = Embedding { vector: vec![1.0, 0.0], dimensions: 2, prompt_tokens: 0 };
        let b = Embedding { vector: vec![1.0, 0.0, 0.0], dimensions: 3, prompt_tokens: 0 };
        assert_eq!(a.cosine_similarity(&b), 0.0);
    }

    #[test]
    fn cosine_zero_vector_returns_zero() {
        let a = Embedding { vector: vec![0.0, 0.0], dimensions: 2, prompt_tokens: 0 };
        let b = Embedding { vector: vec![1.0, 0.0], dimensions: 2, prompt_tokens: 0 };
        assert_eq!(a.cosine_similarity(&b), 0.0);
    }

    #[test]
    fn null_embedding_returns_error() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let null = NullEmbedding;
            assert!(null.embed("test").await.is_err());
        });
    }
}
