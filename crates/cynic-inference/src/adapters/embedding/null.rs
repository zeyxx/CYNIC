use async_trait::async_trait;

use crate::domain::embedding::{Embedding, EmbeddingError};
use crate::ports::embedding_port::EmbeddingPort;

#[derive(Debug)]
pub struct NullEmbedding;

#[async_trait]
impl EmbeddingPort for NullEmbedding {
    async fn embed(&self, _text: &str) -> Result<Embedding, EmbeddingError> {
        Err(EmbeddingError::Unreachable(
            "no embedding server configured".into(),
        ))
    }
}

#[derive(Debug)]
pub struct FixedEmbedding {
    vector: Vec<f32>,
}

impl FixedEmbedding {
    pub fn new(vector: Vec<f32>) -> Self {
        Self { vector }
    }
}

#[async_trait]
impl EmbeddingPort for FixedEmbedding {
    async fn embed(&self, _text: &str) -> Result<Embedding, EmbeddingError> {
        Ok(Embedding {
            vector: self.vector.clone(),
            dimensions: self.vector.len(),
            prompt_tokens: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_embedding_returns_error() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let null = NullEmbedding;
            assert!(null.embed("test").await.is_err());
        });
    }
}
