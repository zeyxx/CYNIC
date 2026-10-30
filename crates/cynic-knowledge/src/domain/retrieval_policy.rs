//! Declarative Canonical Retrieval Policy & Multi-Profile Sweeps (L1 Specification)
//!
//! Provides pure domain structures for configuring hybrid retrieval pipelines
//! (BM25 lexical + dense embeddings + neural cross-encoder rerank + graph expansion)
//! via declarative canonical contracts (`contracts/retrieval-policy.json`).
//!
//! Enables the operator and automated evaluators to sweep across N parameter combinations
//! without recompiling runtime binaries.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::domain::continuous_fusion::NormalizationStrategy;
use crate::domain::epistemic_grounding::EpistemicAssemblyConfig;

pub const RETRIEVAL_POLICY_SCHEMA_V1: &str = "cynic.retrieval_policy.v1";
pub const RETRIEVAL_POLICY_CONTRACT_PATH: &str = "contracts/retrieval-policy.json";

fn default_rrf_k() -> f64 {
    60.0
}

fn default_graph_max_hops() -> usize {
    2
}

fn default_graph_decay_factor() -> f64 {
    0.70
}

/// Specific parameter profile governing a retrieval run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalProfile {
    /// Human-readable explanation of profile intent and trade-offs.
    #[serde(default)]
    pub description: String,
    /// Number of sparse/dense candidates retrieved at stage 1.
    pub candidate_k: usize,
    /// Continuous convex combination weight between sparse and dense (0.0 = BM25 only, 1.0 = dense only).
    pub fusion_alpha: f64,
    /// Score normalization strategy (e.g. "CandidateMinMax", "AbsoluteBand", "EmpiricalCdf").
    pub normalization: String,
    /// Number of top interleaved candidates sent to the neural cross-encoder (0 = bypass reranking).
    pub rerank_pool_cap: usize,
    /// Timeout in milliseconds for neural reranking HTTP calls.
    pub rerank_timeout_ms: u64,
    /// Maximum hops / expansion count for causal knowledge graph traversal (0 = disabled).
    pub graph_max_expansion: usize,
    /// RRF fusion constant k (Reciprocal Rank Fusion). Higher k flattens rank contributions.
    #[serde(default = "default_rrf_k")]
    pub rrf_k: f64,
    /// Maximum number of graph hops performed during expansion.
    #[serde(default = "default_graph_max_hops")]
    pub graph_max_hops: usize,
    /// Per-hop score decay factor applied during graph expansion.
    #[serde(default = "default_graph_decay_factor")]
    pub graph_decay_factor: f64,
    /// Optional BM25 confidence guard gap: when the relative top1-top2 margin is at or above this value, dense noise cannot demote the lexical winner (None disables).
    #[serde(default)]
    pub bm25_guard_gap: Option<f64>,
    /// Optional dense confidence guard gap: when dense margin is at or above this value, fusion proceeds even under a confident BM25 guard (None disables).
    #[serde(default)]
    pub dense_guard_gap: Option<f64>,
    /// Minimum query term coverage ratio required by epistemic anti-hallucination guard.
    pub min_term_coverage: f64,"
    /// Score threshold where high-confidence neural rerank clears the epistemic guard regardless of vocabulary disparity.
    pub neural_confidence_floor: f64,
    /// Maximum number of cited sources included in the assembled prompt context.
    pub max_sources: usize,
    /// Character budget ceiling for context assembly.
    pub max_context_chars: usize,
    /// Whether to apply "Lost-in-the-Middle" positional reordering within authority tiers.
    pub lost_in_the_middle: bool,
}

impl RetrievalProfile {
    /// Resolves the corresponding `NormalizationStrategy` enum.
    pub fn normalization_strategy(&self) -> NormalizationStrategy {
        match self.normalization.to_lowercase().as_str() {
            "empiricalcdf" | "ecdf" => NormalizationStrategy::EmpiricalCdf,
            "absoluteband" | "absolute" => NormalizationStrategy::AbsoluteBand,
            _ => NormalizationStrategy::CandidateMinMax,
        }
    }

    /// Converts this profile into an `EpistemicAssemblyConfig` for prompt context generation.
    pub fn to_assembly_config(&self, min_confidence: f64) -> EpistemicAssemblyConfig {
        EpistemicAssemblyConfig {
            min_confidence,
            min_term_coverage: self.min_term_coverage,
            neural_confidence_floor: self.neural_confidence_floor,
            max_sources: self.max_sources,
            max_context_chars: self.max_context_chars,
            lost_in_the_middle_reordering: self.lost_in_the_middle,
        }
    }
}

/// Canonical retrieval policy contract containing named profiles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalPolicy {
    pub schema: String,
    pub active_profile: String,
    #[serde(default)]
    pub description: String,
    pub profiles: HashMap<String, RetrievalProfile>,
    #[serde(default)]
    pub extra_domain_noise: Vec<String>,
    #[serde(default)]
    pub extra_stop_words: Vec<String>,
}

impl RetrievalPolicy {
    /// Parses JSON contract string. Pure, zero-I/O.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let policy: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if policy.schema != RETRIEVAL_POLICY_SCHEMA_V1 {
            return Err(format!(
                "unsupported retrieval policy schema: got {}, expected {}",
                policy.schema, RETRIEVAL_POLICY_SCHEMA_V1
            ));
        }
        Ok(policy)
    }

    /// Returns a reference to the active profile declared in the contract.
    pub fn active_profile(&self) -> Result<&RetrievalProfile, String> {
        self.profiles
            .get(&self.active_profile)
            .ok_or_else(|| format!("active profile '{}' not found in policy", self.active_profile))
    }

    /// Resolves a requested profile by name, falling back to the active profile if None.
    pub fn resolve_profile<'a>(&'a self, requested: Option<&str>) -> Result<&'a RetrievalProfile, String> {
        match requested {
            Some(name) => self.profiles.get(name).ok_or_else(|| {
                format!(
                    "requested retrieval profile '{}' not found (available: {:?})",
                    name,
                    self.profiles.keys().collect::<Vec<_>>()
                )
            }),
            None => self.active_profile(),
        }
    }

    /// Built-in fallback policy ensuring zero-downtime execution if the contract file is absent.
    pub fn default_policy() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert(
            "balanced".to_string(),
            RetrievalProfile {
                description: "Calibrated balanced hybrid retrieval: BM25 + Dense Q8 embeddings with neural cross-encoder rerank and topological graph expansion.".to_string(),
                candidate_k: 50,
                fusion_alpha: 0.80,
                normalization: "CandidateMinMax".to_string(),
                rerank_pool_cap: 24,
                rerank_timeout_ms: 15000,
                graph_max_expansion: 2,
                rrf_k: 60.0,
                graph_max_hops: 2,
                graph_decay_factor: 0.70,
                bm25_guard_gap: Some(0.25),
                dense_guard_gap: Some(0.15),
                min_term_coverage: 0.30,
                neural_confidence_floor: 0.70,
                max_sources: 5,
                max_context_chars: 8000,
                lost_in_the_middle: false,
            },
        );

        Self {
            schema: RETRIEVAL_POLICY_SCHEMA_V1.to_string(),
            active_profile: "balanced".to_string(),
            description: "Default sovereign retrieval policy".to_string(),
            profiles,
            extra_domain_noise: Vec::new(),
            extra_stop_words: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_canonical_retrieval_policy() {
        let json = r#"{
            "schema": "cynic.retrieval_policy.v1",
            "active_profile": "balanced",
            "profiles": {
                "balanced": {
                    "candidate_k": 50,
                    "fusion_alpha": 0.80,
                    "normalization": "CandidateMinMax",
                    "rerank_pool_cap": 24,
                    "rerank_timeout_ms": 15000,
                    "graph_max_expansion": 2,
                    "min_term_coverage": 0.30,
                    "neural_confidence_floor": 0.70,
                    "max_sources": 5,
                    "max_context_chars": 8000,
                    "lost_in_the_middle": false
                },
                "high_precision": {
                    "candidate_k": 30,
                    "fusion_alpha": 0.50,
                    "normalization": "EmpiricalCdf",
                    "rerank_pool_cap": 16,
                    "rerank_timeout_ms": 10000,
                    "graph_max_expansion": 1,
                    "min_term_coverage": 0.50,
                    "neural_confidence_floor": 0.75,
                    "max_sources": 3,
                    "max_context_chars": 6000,
                    "lost_in_the_middle": true
                }
            }
        }"#;

        let policy = RetrievalPolicy::from_json(json).expect("valid retrieval policy");
        assert_eq!(policy.active_profile, "balanced");

        let active = policy.active_profile().expect("active profile exists");
        assert_eq!(active.candidate_k, 50);
        assert_eq!(active.fusion_alpha, 0.80);
        assert_eq!(active.normalization_strategy(), NormalizationStrategy::CandidateMinMax);

        let hp = policy.resolve_profile(Some("high_precision")).expect("high_precision exists");
        assert_eq!(hp.candidate_k, 30);
        assert_eq!(hp.fusion_alpha, 0.50);
        assert_eq!(hp.normalization_strategy(), NormalizationStrategy::EmpiricalCdf);
        assert!(hp.lost_in_the_middle);

        assert!(policy.resolve_profile(Some("unknown_profile")).is_err());
    }

    #[test]
    fn test_rejects_wrong_schema() {
        let json = r#"{"schema": "invalid.v1", "active_profile": "balanced", "profiles": {}}"#;
        assert!(RetrievalPolicy::from_json(json).is_err());
    }
}
