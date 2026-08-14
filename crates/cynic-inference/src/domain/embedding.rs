//! Embedding domain values and errors.

#[derive(Debug, Clone)]
pub struct Embedding {
    pub vector: Vec<f32>,
    pub dimensions: usize,
    pub prompt_tokens: u32,
}

impl Embedding {
    pub fn cosine_similarity(&self, other: &Embedding) -> f64 {
        if self.vector.len() != other.vector.len() {
            return 0.0;
        }
        let dot: f64 = self
            .vector
            .iter()
            .zip(&other.vector)
            .map(|(a, b)| *a as f64 * *b as f64)
            .sum();
        let norm_a: f64 = self
            .vector
            .iter()
            .map(|value| (*value as f64).powi(2))
            .sum::<f64>()
            .sqrt();
        let norm_b: f64 = other
            .vector
            .iter()
            .map(|value| (*value as f64).powi(2))
            .sum::<f64>()
            .sqrt();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_vectors() {
        let a = Embedding {
            vector: vec![1.0, 0.0, 0.0],
            dimensions: 3,
            prompt_tokens: 0,
        };
        let b = Embedding {
            vector: vec![1.0, 0.0, 0.0],
            dimensions: 3,
            prompt_tokens: 0,
        };
        assert!((a.cosine_similarity(&b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = Embedding {
            vector: vec![1.0, 0.0],
            dimensions: 2,
            prompt_tokens: 0,
        };
        let b = Embedding {
            vector: vec![0.0, 1.0],
            dimensions: 2,
            prompt_tokens: 0,
        };
        assert!(a.cosine_similarity(&b).abs() < 1e-6);
    }

    #[test]
    fn cosine_opposite_vectors() {
        let a = Embedding {
            vector: vec![1.0, 0.0],
            dimensions: 2,
            prompt_tokens: 0,
        };
        let b = Embedding {
            vector: vec![-1.0, 0.0],
            dimensions: 2,
            prompt_tokens: 0,
        };
        assert!((a.cosine_similarity(&b) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_different_lengths_returns_zero() {
        let a = Embedding {
            vector: vec![1.0, 0.0],
            dimensions: 2,
            prompt_tokens: 0,
        };
        let b = Embedding {
            vector: vec![1.0, 0.0, 0.0],
            dimensions: 3,
            prompt_tokens: 0,
        };
        assert_eq!(a.cosine_similarity(&b), 0.0);
    }

    #[test]
    fn cosine_zero_vector_returns_zero() {
        let a = Embedding {
            vector: vec![0.0, 0.0],
            dimensions: 2,
            prompt_tokens: 0,
        };
        let b = Embedding {
            vector: vec![1.0, 0.0],
            dimensions: 2,
            prompt_tokens: 0,
        };
        assert_eq!(a.cosine_similarity(&b), 0.0);
    }
}
