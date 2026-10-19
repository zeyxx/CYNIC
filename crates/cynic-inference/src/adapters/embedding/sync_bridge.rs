//! Sync bridge: serve `cynic-knowledge`'s sync `ChunkEmbedder` port from any
//! async `EmbeddingPort` backend, on a private current-thread runtime.
//!
//! The runtime lives here (provider side) so `cynic-knowledge` stays free of
//! async runtimes. Dimensions are validated once at construction: a backend
//! that disagrees with itself fails here, not mid-corpus.

use crate::ports::embedding_port::EmbeddingPort;

/// Sync `ChunkEmbedder` over an async embedding backend.
pub struct SyncEmbeddingBridge<B: EmbeddingPort> {
    backend: B,
    runtime: tokio::runtime::Runtime,
    dimensions: usize,
}

impl<B: EmbeddingPort> std::fmt::Debug for SyncEmbeddingBridge<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncEmbeddingBridge")
            .field("dimensions", &self.dimensions)
            .finish_non_exhaustive()
    }
}

impl<B: EmbeddingPort> SyncEmbeddingBridge<B> {
    /// Probes the backend once (`probe_text`) to pin the vector dimension.
    pub fn probe(
        backend: B,
        probe_text: &str,
    ) -> Result<Self, crate::domain::embedding::EmbeddingError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                crate::domain::embedding::EmbeddingError::Unreachable(format!(
                    "sync bridge runtime failed: {e}"
                ))
            })?;
        let probe = runtime.block_on(backend.embed(probe_text))?;
        let dimensions = probe.vector.len();
        if dimensions == 0 {
            return Err(crate::domain::embedding::EmbeddingError::Protocol(
                "sync bridge probe returned an empty vector".to_string(),
            ));
        }
        Ok(Self {
            backend,
            runtime,
            dimensions,
        })
    }

    /// Pinned vector dimension observed at construction.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}

impl<B: EmbeddingPort> cynic_knowledge::ChunkEmbedder for SyncEmbeddingBridge<B> {
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        self.embed_batch_reported(texts, &|_, _| {})
    }
}

impl<B: EmbeddingPort> SyncEmbeddingBridge<B> {
    /// `embed_batch_reported` trait impl: whole slice in one call so cluster
    /// fanout keeps a single shared queue (chunking starves endpoints);
    /// `report` fires per completed sublot from the workers.
    pub fn embed_batch_reported(
        &self,
        texts: &[&str],
        report: &(dyn Fn(usize, usize) + Sync),
    ) -> Result<Vec<Vec<f32>>, String> {
        let embeddings = self
            .runtime
            .block_on(self.backend.embed_batch_fanout(texts, report))
            .map_err(|e| e.to_string())?;
        for embedding in &embeddings {
            if embedding.vector.len() != self.dimensions {
                return Err(format!(
                    "sync bridge dimension drift: expected {}, got {}",
                    self.dimensions,
                    embedding.vector.len()
                ));
            }
        }
        Ok(embeddings.into_iter().map(|e| e.vector).collect())
    }

    /// Resilient embedding for evaluation harnesses: total by construction.
    /// Preserves successfully calculated vectors even if isolated lots fail,
    /// returning per-item results and an explicit manifest of per-lot failures.
    pub fn embed_batch_resilient(
        &self,
        texts: &[&str],
        report: &(dyn Fn(usize, usize) + Sync),
    ) -> (Vec<Option<Vec<f32>>>, Vec<(usize, String)>) {
        use super::http::lot_size;
        let mut vectors = Vec::with_capacity(texts.len());
        let mut recorded_errors = Vec::new();

        for (lot_idx, lot) in texts.chunks(lot_size()).enumerate() {
            match self.runtime.block_on(self.backend.embed_batch(lot)) {
                Ok(embeddings) => {
                    for embedding in embeddings {
                        if embedding.vector.len() != self.dimensions {
                            recorded_errors.push((
                                lot_idx,
                                format!(
                                    "sync bridge dimension drift: expected {}, got {}",
                                    self.dimensions,
                                    embedding.vector.len()
                                ),
                            ));
                            vectors.push(None);
                        } else {
                            vectors.push(Some(embedding.vector));
                        }
                    }
                }
                Err(e) => {
                    recorded_errors.push((lot_idx, e.to_string()));
                    for _ in 0..lot.len() {
                        vectors.push(None);
                    }
                }
            }
            report(vectors.iter().filter(|v| v.is_some()).count(), texts.len());
        }
        (vectors, recorded_errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::embedding::{Embedding, EmbeddingError};
    use async_trait::async_trait;
    use cynic_knowledge::ChunkEmbedder;

    struct FixedBackend {
        dimensions: usize,
    }

    #[async_trait]
    impl EmbeddingPort for FixedBackend {
        async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
            Ok(Embedding {
                vector: vec![text.len() as f32; self.dimensions],
                dimensions: self.dimensions,
                prompt_tokens: None,
            })
        }
    }

    #[test]
    fn bridge_pins_dimensions_and_embeds_batches() {
        let bridge =
            SyncEmbeddingBridge::probe(FixedBackend { dimensions: 4 }, "probe").expect("probe");
        assert_eq!(bridge.dimensions(), 4);
        let vectors = bridge.embed_batch(&["a", "abc"]).expect("batch");
        assert_eq!(vectors.len(), 2);
        assert_eq!(vectors[0], vec![1.0; 4]);
        assert_eq!(vectors[1], vec![3.0; 4]);
    }

    #[test]
    fn bridge_rejects_empty_probe_vector() {
        assert!(SyncEmbeddingBridge::probe(FixedBackend { dimensions: 0 }, "probe").is_err());
    }

    #[test]
    fn bridge_reports_progress_ending_at_total() {
        let bridge =
            SyncEmbeddingBridge::probe(FixedBackend { dimensions: 2 }, "probe").expect("probe");
        let texts: Vec<&str> = (0..70).map(|_| "abc").collect();
        let seen = std::sync::Mutex::new(Vec::new());
        let vectors = bridge
            .embed_batch_reported(&texts, &|done, total| {
                seen.lock().unwrap().push((done, total));
            })
            .expect("reported batch");
        assert_eq!(vectors.len(), 70);
        let seen = seen.lock().unwrap();
        assert!(!seen.is_empty(), "progress must fire at least once");
        assert_eq!(*seen.last().unwrap(), (70, 70));
        for (done, total) in seen.iter() {
            assert!(*done <= *total, "progress must never exceed total");
        }
    }

    #[test]
    fn bridge_rejects_dimension_drift() {
        struct DriftingBackend;
        #[async_trait]
        impl EmbeddingPort for DriftingBackend {
            async fn embed(&self, _text: &str) -> Result<Embedding, EmbeddingError> {
                Ok(Embedding {
                    vector: vec![1.0; 2],
                    dimensions: 2,
                    prompt_tokens: None,
                })
            }
            async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
                let mut out = Vec::new();
                for (i, text) in texts.iter().enumerate() {
                    let dims = if i == 0 { 2 } else { 3 };
                    out.push(Embedding {
                        vector: vec![text.len() as f32; dims],
                        dimensions: dims,
                        prompt_tokens: None,
                    });
                }
                Ok(out)
            }
        }
        let bridge =
            SyncEmbeddingBridge::probe(DriftingBackend, "probe").expect("probe pins dim 2");
        assert!(bridge.embed_batch(&["a", "b"]).is_err());
    }
}
