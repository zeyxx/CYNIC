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
