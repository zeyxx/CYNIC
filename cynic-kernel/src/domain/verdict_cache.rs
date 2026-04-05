//! Semantic verdict cache — cosine similarity lookup before Dog fan-out.
//! Pure domain: depends only on domain types (Embedding, Verdict). Zero external deps.
//!
//! When a stimulus arrives, embed it and compare against cached embeddings.
//! Cosine > threshold → return cached verdict (0 API calls, 0 tokens).
//! Miss → evaluate normally, then store embedding + verdict for future lookups.

use crate::domain::dog::Verdict;
use crate::domain::embedding::Embedding;
use std::collections::VecDeque;
use std::sync::RwLock;

/// φ⁻² + φ⁻⁴ = 0.528 as cache hit threshold would be too loose.
/// 0.95 is a conservative threshold: only near-identical stimuli get cached.
const CACHE_HIT_THRESHOLD: f64 = 0.95;

/// Max cached entries before FIFO eviction.
const MAX_ENTRIES: usize = 1000;

/// T6: Cache context — domain + Dogs configuration hash.
/// A cache hit requires BOTH cosine similarity ≥ threshold AND matching CacheContext.
/// This prevents cross-domain contamination (F17) and stale Dog config hits (F19).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheContext {
    domain: String,
    dogs_hash: u64,
}

impl CacheContext {
    /// Create from domain hint and Dogs hash (from Judge::available_dogs_hash).
    pub fn new(domain: &str, dogs_hash: u64) -> Self {
        Self {
            domain: domain.to_string(),
            dogs_hash,
        }
    }
}

struct CacheEntry {
    embedding: Embedding,
    verdict: Verdict,
    context: CacheContext,
}

pub struct VerdictCache {
    entries: RwLock<VecDeque<CacheEntry>>,
    threshold: f64,
}

impl std::fmt::Debug for VerdictCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerdictCache").finish_non_exhaustive()
    }
}

/// Result of a cache lookup.
#[derive(Debug)]
pub enum CacheLookup {
    /// Found a similar verdict — return it directly.
    Hit {
        verdict: Box<Verdict>,
        similarity: f64,
    },
    /// No match above threshold.
    Miss,
}

impl Default for VerdictCache {
    fn default() -> Self {
        Self::new()
    }
}

impl VerdictCache {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(VecDeque::new()),
            threshold: CACHE_HIT_THRESHOLD,
        }
    }

    /// Search for a cached verdict similar to the given embedding.
    /// T6: Only matches entries with the SAME CacheContext (domain + dogs_hash).
    /// This prevents cross-domain hits (F17) and stale Dog config hits (F19).
    pub fn lookup(&self, query: &Embedding, ctx: &CacheContext) -> CacheLookup {
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());

        let mut best_sim = 0.0_f64;
        let mut best_idx = None;

        for (i, entry) in entries.iter().enumerate() {
            // T6: skip entries with different domain or Dog configuration
            if entry.context != *ctx {
                continue;
            }
            let sim = query.cosine_similarity(&entry.embedding);
            if sim > best_sim {
                best_sim = sim;
                best_idx = Some(i);
            }
        }

        if best_sim >= self.threshold
            && let Some(idx) = best_idx
        {
            return CacheLookup::Hit {
                verdict: Box::new(entries[idx].verdict.clone()),
                similarity: best_sim,
            };
        }

        CacheLookup::Miss
    }

    /// Store a verdict with its embedding and context for future lookups.
    /// T6: CacheContext ensures the verdict is only returned for matching domain + Dog config.
    /// FIFO eviction when at capacity.
    pub fn store(&self, embedding: Embedding, verdict: Verdict, ctx: CacheContext) {
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());

        if entries.len() >= MAX_ENTRIES {
            entries.pop_front(); // O(1) FIFO eviction
        }

        entries.push_back(CacheEntry {
            embedding,
            verdict,
            context: ctx,
        });
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
        Embedding {
            vector: vals,
            dimensions: dims,
            prompt_tokens: 0,
        }
    }

    fn test_ctx() -> CacheContext {
        CacheContext::new("test", 12345)
    }

    fn other_ctx() -> CacheContext {
        CacheContext::new("other-domain", 99999)
    }

    fn make_verdict(id: &str) -> Verdict {
        Verdict {
            id: id.to_string(),
            domain: "test".to_string(),
            kind: VerdictKind::Wag,
            q_score: QScore {
                total: 0.5,
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
            },
            reasoning: AxiomReasoning::default(),
            dog_id: "test".to_string(),
            stimulus_summary: "test".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            dog_scores: vec![],
            anomaly_detected: false,
            max_disagreement: 0.0,
            anomaly_axiom: None,
            voter_count: 0,
            failed_dogs: vec![],
            failed_dog_errors: Default::default(),
            integrity_hash: None,
            prev_hash: None,
        }
    }

    #[test]
    fn miss_on_empty_cache() {
        let cache = VerdictCache::new();
        let query = make_embedding(vec![1.0, 0.0, 0.0]);
        assert!(matches!(
            cache.lookup(&query, &test_ctx()),
            CacheLookup::Miss
        ));
    }

    #[test]
    fn hit_on_identical_embedding() {
        let cache = VerdictCache::new();
        let emb = make_embedding(vec![1.0, 0.0, 0.0]);
        cache.store(emb, make_verdict("v1"), test_ctx());

        let query = make_embedding(vec![1.0, 0.0, 0.0]);
        match cache.lookup(&query, &test_ctx()) {
            CacheLookup::Hit {
                ref verdict,
                similarity,
            } => {
                assert_eq!(verdict.id, "v1");
                assert!((similarity - 1.0).abs() < 1e-6);
            }
            CacheLookup::Miss => panic!("Expected hit on identical embedding"),
        }
    }

    #[test]
    fn miss_on_orthogonal_embedding() {
        let cache = VerdictCache::new();
        cache.store(
            make_embedding(vec![1.0, 0.0, 0.0]),
            make_verdict("v1"),
            test_ctx(),
        );

        let query = make_embedding(vec![0.0, 1.0, 0.0]);
        assert!(matches!(
            cache.lookup(&query, &test_ctx()),
            CacheLookup::Miss
        ));
    }

    #[test]
    fn hit_on_near_identical() {
        let cache = VerdictCache::new();
        cache.store(
            make_embedding(vec![1.0, 0.0, 0.0]),
            make_verdict("v1"),
            test_ctx(),
        );

        let query = make_embedding(vec![1.0, 0.05, 0.0]);
        match cache.lookup(&query, &test_ctx()) {
            CacheLookup::Hit { similarity, .. } => {
                assert!(
                    similarity > 0.95,
                    "Expected high similarity, got {similarity}"
                );
            }
            CacheLookup::Miss => panic!("Expected hit on near-identical embedding"),
        }
    }

    #[test]
    fn fifo_eviction() {
        let cache = VerdictCache::new();
        let ctx = test_ctx();
        for i in 0..1001 {
            let mut v = vec![0.0_f32; 3];
            v[i % 3] = 1.0;
            cache.store(
                make_embedding(v),
                make_verdict(&format!("v{i}")),
                ctx.clone(),
            );
        }
        assert_eq!(cache.len(), MAX_ENTRIES);
    }

    #[test]
    fn best_match_wins() {
        let cache = VerdictCache::new();
        let ctx = test_ctx();
        cache.store(
            make_embedding(vec![1.0, 0.0, 0.0]),
            make_verdict("exact"),
            ctx.clone(),
        );
        cache.store(
            make_embedding(vec![0.9, 0.1, 0.0]),
            make_verdict("close"),
            ctx.clone(),
        );

        let query = make_embedding(vec![1.0, 0.0, 0.0]);
        match cache.lookup(&query, &ctx) {
            CacheLookup::Hit { verdict, .. } => {
                assert_eq!(verdict.id, "exact");
            }
            CacheLookup::Miss => panic!("Expected hit"),
        }
    }

    // ── T6 CONTRACT TESTS ────────────────────────────────

    #[test]
    fn miss_on_different_domain() {
        // F17: a verdict cached for "chess" must NOT serve a "code" query
        let cache = VerdictCache::new();
        cache.store(
            make_embedding(vec![1.0, 0.0, 0.0]),
            make_verdict("chess-v"),
            test_ctx(), // domain="test"
        );

        let query = make_embedding(vec![1.0, 0.0, 0.0]); // identical embedding
        assert!(
            matches!(cache.lookup(&query, &other_ctx()), CacheLookup::Miss),
            "F17: cross-domain cache hit must not happen"
        );
    }

    #[test]
    fn miss_on_different_dogs_hash() {
        // F19: a verdict from 5 Dogs must NOT serve when only 2 are available
        let cache = VerdictCache::new();
        let ctx_5dogs = CacheContext::new("chess", 55555);
        cache.store(
            make_embedding(vec![1.0, 0.0, 0.0]),
            make_verdict("5dog-v"),
            ctx_5dogs,
        );

        let ctx_2dogs = CacheContext::new("chess", 22222); // different dogs_hash
        let query = make_embedding(vec![1.0, 0.0, 0.0]);
        assert!(
            matches!(cache.lookup(&query, &ctx_2dogs), CacheLookup::Miss),
            "F19: cache hit with different Dog config must not happen"
        );
    }

    #[test]
    fn hit_on_same_context() {
        // Same embedding + same context → cache hit
        let cache = VerdictCache::new();
        let ctx = CacheContext::new("chess", 55555);
        cache.store(
            make_embedding(vec![1.0, 0.0, 0.0]),
            make_verdict("match"),
            ctx.clone(),
        );

        let query = make_embedding(vec![1.0, 0.0, 0.0]);
        match cache.lookup(&query, &ctx) {
            CacheLookup::Hit { verdict, .. } => assert_eq!(verdict.id, "match"),
            CacheLookup::Miss => panic!("Expected hit with same context"),
        }
    }
}
