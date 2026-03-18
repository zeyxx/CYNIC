//! Semantic verdict cache — cosine similarity lookup before Dog fan-out.
//! Pure domain: depends only on domain types (Embedding, Verdict). Zero external deps.
//!
//! When a stimulus arrives, embed it and compare against cached embeddings.
//! Cosine > threshold → return cached verdict (0 API calls, 0 tokens).
//! Miss → evaluate normally, then store embedding + verdict for future lookups.

use crate::domain::dog::Verdict;
use crate::domain::embedding::Embedding;
use std::sync::RwLock;

/// φ⁻² + φ⁻⁴ = 0.528 as cache hit threshold would be too loose.
/// 0.95 is a conservative threshold: only near-identical stimuli get cached.
const CACHE_HIT_THRESHOLD: f64 = 0.95;

/// Max cached entries before FIFO eviction.
const MAX_ENTRIES: usize = 1000;

struct CacheEntry {
    embedding: Embedding,
    verdict: Verdict,
}

pub struct VerdictCache {
    entries: RwLock<Vec<CacheEntry>>,
    threshold: f64,
}

/// Result of a cache lookup.
pub enum CacheLookup {
    /// Found a similar verdict — return it directly.
    Hit { verdict: Box<Verdict>, similarity: f64 },
    /// No match above threshold.
    Miss,
}

impl Default for VerdictCache {
    fn default() -> Self { Self::new() }
}

impl VerdictCache {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            threshold: CACHE_HIT_THRESHOLD,
        }
    }

    /// Search for a cached verdict similar to the given embedding.
    /// Returns the best match above threshold, or Miss.
    pub fn lookup(&self, query: &Embedding) -> CacheLookup {
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());

        let mut best_sim = 0.0_f64;
        let mut best_idx = None;

        for (i, entry) in entries.iter().enumerate() {
            let sim = query.cosine_similarity(&entry.embedding);
            if sim > best_sim {
                best_sim = sim;
                best_idx = Some(i);
            }
        }

        if best_sim >= self.threshold && let Some(idx) = best_idx {
            return CacheLookup::Hit {
                verdict: Box::new(entries[idx].verdict.clone()),
                similarity: best_sim,
            };
        }

        CacheLookup::Miss
    }

    /// Store a verdict with its embedding for future lookups.
    /// FIFO eviction when at capacity.
    pub fn store(&self, embedding: Embedding, verdict: Verdict) {
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());

        if entries.len() >= MAX_ENTRIES {
            entries.remove(0); // FIFO: drop oldest
        }

        entries.push(CacheEntry { embedding, verdict });
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dog::*;

    fn make_embedding(vals: Vec<f32>) -> Embedding {
        let dims = vals.len();
        Embedding { vector: vals, dimensions: dims, prompt_tokens: 0 }
    }

    fn make_verdict(id: &str) -> Verdict {
        Verdict {
            id: id.to_string(),
            kind: VerdictKind::Wag,
            q_score: QScore { total: 0.5, fidelity: 0.5, phi: 0.5, verify: 0.5, culture: 0.5, burn: 0.5, sovereignty: 0.5 },
            reasoning: AxiomReasoning::default(),
            dog_id: "test".to_string(),
            stimulus_summary: "test".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            dog_scores: vec![],
            anomaly_detected: false,
            max_disagreement: 0.0,
            anomaly_axiom: None,
            failed_dogs: vec![],
            integrity_hash: None,
            prev_hash: None,
        }
    }

    #[test]
    fn miss_on_empty_cache() {
        let cache = VerdictCache::new();
        let query = make_embedding(vec![1.0, 0.0, 0.0]);
        assert!(matches!(cache.lookup(&query), CacheLookup::Miss));
    }

    #[test]
    fn hit_on_identical_embedding() {
        let cache = VerdictCache::new();
        let emb = make_embedding(vec![1.0, 0.0, 0.0]);
        cache.store(emb.clone(), make_verdict("v1"));

        let query = make_embedding(vec![1.0, 0.0, 0.0]);
        match cache.lookup(&query) {
            CacheLookup::Hit { ref verdict, similarity } => {
                assert_eq!(verdict.id, "v1");
                assert!((similarity - 1.0).abs() < 1e-6);
            }
            CacheLookup::Miss => panic!("Expected hit on identical embedding"),
        }
    }

    #[test]
    fn miss_on_orthogonal_embedding() {
        let cache = VerdictCache::new();
        cache.store(make_embedding(vec![1.0, 0.0, 0.0]), make_verdict("v1"));

        let query = make_embedding(vec![0.0, 1.0, 0.0]);
        assert!(matches!(cache.lookup(&query), CacheLookup::Miss));
    }

    #[test]
    fn hit_on_near_identical() {
        let cache = VerdictCache::new();
        cache.store(make_embedding(vec![1.0, 0.0, 0.0]), make_verdict("v1"));

        // Very slight variation — cosine should be > 0.95
        let query = make_embedding(vec![1.0, 0.05, 0.0]);
        match cache.lookup(&query) {
            CacheLookup::Hit { similarity, .. } => {
                assert!(similarity > 0.95, "Expected high similarity, got {}", similarity);
            }
            CacheLookup::Miss => panic!("Expected hit on near-identical embedding"),
        }
    }

    #[test]
    fn fifo_eviction() {
        let cache = VerdictCache::new();
        // Fill beyond capacity
        for i in 0..1001 {
            let mut v = vec![0.0_f32; 3];
            v[i % 3] = 1.0;
            cache.store(make_embedding(v), make_verdict(&format!("v{}", i)));
        }
        assert_eq!(cache.len(), MAX_ENTRIES);
    }

    #[test]
    fn best_match_wins() {
        let cache = VerdictCache::new();
        cache.store(make_embedding(vec![1.0, 0.0, 0.0]), make_verdict("exact"));
        cache.store(make_embedding(vec![0.9, 0.1, 0.0]), make_verdict("close"));

        let query = make_embedding(vec![1.0, 0.0, 0.0]);
        match cache.lookup(&query) {
            CacheLookup::Hit { verdict, .. } => {
                assert_eq!(verdict.id, "exact");
            }
            CacheLookup::Miss => panic!("Expected hit"),
        }
    }
}
