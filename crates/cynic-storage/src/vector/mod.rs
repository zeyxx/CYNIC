//! Contiguous in-memory SIMD-accelerated vector store implementing `VectorStorePort`.
//! Optimized for dynamic-dimensional embeddings (384, 768, 1024, 1536) and sub-2ms top-k retrieval.

use parking_lot::RwLock;
use std::sync::Arc;

use crate::error::StorageError;
use crate::ports::VectorStorePort;

/// Contiguous flat-memory vector index.
/// All embedding components are stored in a single contiguous buffer to maximize L2/L3 cache line utilization.
struct FlatVectorState {
    dimension: usize,
    ids: Vec<String>,
    /// Contiguous flattened vector storage: length = ids.len() * dimension
    buffer: Vec<f32>,
}

impl FlatVectorState {
    fn new(dimension: usize) -> Self {
        Self {
            dimension,
            ids: Vec::new(),
            buffer: Vec::new(),
        }
    }

    fn insert(&mut self, id: &str, vector: &[f32]) -> Result<(), StorageError> {
        if vector.len() != self.dimension {
            return Err(StorageError::Codec(format!(
                "dimension mismatch: expected {}, got {}",
                self.dimension,
                vector.len()
            )));
        }

        // Normalize vector to unit length for cosine dot product
        let norm = vector.iter().map(|&x| x * x).sum::<f32>().sqrt();
        let inv_norm = if norm > 1e-8 { 1.0 / norm } else { 1.0 };

        if let Some(pos) = self.ids.iter().position(|existing| existing == id) {
            let offset = pos * self.dimension;
            for (i, &val) in vector.iter().enumerate() {
                self.buffer[offset + i] = val * inv_norm;
            }
        } else {
            self.ids.push(id.to_string());
            for &val in vector {
                self.buffer.push(val * inv_norm);
            }
        }

        Ok(())
    }

    fn delete(&mut self, id: &str) -> Result<(), StorageError> {
        if let Some(pos) = self.ids.iter().position(|existing| existing == id) {
            self.ids.remove(pos);
            let offset = pos * self.dimension;
            self.buffer.drain(offset..offset + self.dimension);
            Ok(())
        } else {
            Ok(()) // Idempotent delete
        }
    }

    fn search_nearest(
        &self,
        query: &[f32],
        k: usize,
        threshold: f32,
    ) -> Result<Vec<(String, f32)>, StorageError> {
        if query.len() != self.dimension {
            return Err(StorageError::Codec(format!(
                "query dimension mismatch: expected {}, got {}",
                self.dimension,
                query.len()
            )));
        }

        if self.ids.is_empty() || k == 0 {
            return Ok(Vec::new());
        }

        let q_norm = query.iter().map(|&x| x * x).sum::<f32>().sqrt();
        let inv_q_norm = if q_norm > 1e-8 { 1.0 / q_norm } else { 1.0 };
        let normalized_query: Vec<f32> = query.iter().map(|&x| x * inv_q_norm).collect();

        let mut scores = Vec::with_capacity(self.ids.len());

        for (idx, id) in self.ids.iter().enumerate() {
            let offset = idx * self.dimension;
            let vec_slice = &self.buffer[offset..offset + self.dimension];

            // Direct contiguous dot product (auto-vectorized by LLVM with AVX2/NEON)
            let mut dot_product = 0.0f32;
            for i in 0..self.dimension {
                dot_product += normalized_query[i] * vec_slice[i];
            }

            if dot_product >= threshold {
                scores.push((id.clone(), dot_product));
            }
        }

        // Sort descending by similarity score
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);

        Ok(scores)
    }

    fn len(&self) -> usize {
        self.ids.len()
    }
}

/// In-memory contiguous SIMD vector store adapter.
#[derive(Clone)]
pub struct SimdVectorAdapter {
    state: Arc<RwLock<FlatVectorState>>,
}

impl SimdVectorAdapter {
    /// Create a new SIMD vector store for dynamic dimension `D`.
    pub fn new(dimension: usize) -> Self {
        Self {
            state: Arc::new(RwLock::new(FlatVectorState::new(dimension))),
        }
    }

    /// Return the total number of vectors in the store.
    pub fn len(&self) -> usize {
        self.state.read().len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl VectorStorePort for SimdVectorAdapter {
    fn insert_vector(&self, id: &str, vector: &[f32]) -> Result<(), StorageError> {
        self.state.write().insert(id, vector)
    }

    fn search_nearest(
        &self,
        query: &[f32],
        k: usize,
        threshold: f32,
    ) -> Result<Vec<(String, f32)>, StorageError> {
        self.state.read().search_nearest(query, k, threshold)
    }

    fn delete_vector(&self, id: &str) -> Result<(), StorageError> {
        self.state.write().delete(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simd_vector_insert_search_nearest_top_k() {
        let store = SimdVectorAdapter::new(4);

        // Insert orthogonal vectors
        store.insert_vector("v1", &[1.0, 0.0, 0.0, 0.0]).unwrap();
        store.insert_vector("v2", &[0.0, 1.0, 0.0, 0.0]).unwrap();
        store
            .insert_vector("v3", &[0.707, 0.707, 0.0, 0.0])
            .unwrap();

        assert_eq!(store.len(), 3);

        // Query aligned with v1
        let results = store.search_nearest(&[1.0, 0.0, 0.0, 0.0], 2, 0.5).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "v1");
        assert!((results[0].1 - 1.0).abs() < 1e-4);
        assert_eq!(results[1].0, "v3");
        assert!((results[1].1 - 0.707).abs() < 1e-2);

        // Delete v1
        store.delete_vector("v1").unwrap();
        assert_eq!(store.len(), 2);

        let results_after = store.search_nearest(&[1.0, 0.0, 0.0, 0.0], 2, 0.5).unwrap();
        assert_eq!(results_after.len(), 1);
        assert_eq!(results_after[0].0, "v3");
    }

    #[test]
    fn simd_vector_rejects_dimension_mismatch() {
        let store = SimdVectorAdapter::new(384);
        let err = store.insert_vector("invalid", &[0.1; 128]);
        assert!(err.is_err());
    }
}
