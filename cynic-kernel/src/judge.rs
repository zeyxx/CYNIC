//! Judge — orchestrates Dogs, computes consensus, emits Verdicts.
//! Parallel evaluation via futures_util::future::join_all.
//! Residual detection: disagreement > φ⁻² = ANOMALY.

use crate::domain::dog::{estimate_tokens, *};
use crate::domain::health_gate::HealthGate;
use crate::domain::metrics::Metrics;
use crate::organ::health::{DogStats, ScoreFailureKind};
use crate::organ::{BackendHandle, InferenceOrgan, ScoreOutcome};
use chrono::Utc;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct Judge {
    dogs: Vec<Box<dyn Dog>>,
    breakers: Vec<Arc<dyn HealthGate>>,
    /// Organ handles — one per dog (same index). None for dogs without organ tracking.
    organ_handles: Vec<Option<BackendHandle>>,
    /// Hash of the last verdict — forms the chain. Protected by Mutex for concurrent access.
    last_hash: Mutex<Option<String>>,
}

impl std::fmt::Debug for Judge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Judge").finish_non_exhaustive()
    }
}

impl Judge {
    pub fn new(dogs: Vec<Box<dyn Dog>>, breakers: Vec<Arc<dyn HealthGate>>) -> Self {
        assert_eq!(dogs.len(), breakers.len(), "dogs and breakers must be 1:1");
        let n = dogs.len();
        Self {
            dogs,
            breakers,
            organ_handles: vec![None; n],
            last_hash: Mutex::new(None),
        }
    }

    /// Attach organ backend handles (one per dog, same index). Call after `new()`.
    /// Handles that are `None` are skipped — dogs without organ tracking still work.
    pub fn with_organ_handles(mut self, handles: Vec<Option<BackendHandle>>) -> Self {
        assert_eq!(
            handles.len(),
            self.dogs.len(),
            "organ_handles must be 1:1 with dogs"
        );
        self.organ_handles = handles;
        self
    }

    /// Seed the hash chain from the last stored verdict (call at boot).
    pub fn seed_chain(&self, prev_hash: Option<String>) {
        if let Ok(mut lock) = self.last_hash.lock() {
            *lock = prev_hash;
        }
    }

    /// Return list of available Dog IDs.
    pub fn dog_ids(&self) -> Vec<String> {
        self.dogs.iter().map(|d| d.id().to_string()).collect()
    }

    /// Hash of Dogs that would participate in evaluation (filter + CB check).
    /// Used by VerdictCache to invalidate entries when Dog configuration changes.
    /// FNV-1a hash of sorted Dog IDs that pass filter + circuit breaker.
    pub fn available_dogs_hash(&self, filter: Option<&[String]>) -> u64 {
        let mut ids: Vec<&str> = match filter {
            Some(f) => self
                .dogs
                .iter()
                .zip(self.breakers.iter())
                .filter(|(d, cb)| {
                    cb.should_allow()
                        && (d.id() == "deterministic-dog" || f.iter().any(|id| id == d.id()))
                })
                .map(|(d, _)| d.id())
                .collect(),
            None => self
                .dogs
                .iter()
                .zip(self.breakers.iter())
                .filter(|(_, cb)| cb.should_allow())
                .map(|(d, _)| d.id())
                .collect(),
        };
        ids.sort_unstable();
        let joined = ids.join("+");
        // FNV-1a (same as ccm::content_hash)
        let mut h: u64 = 0xcbf29ce484222325;
        for byte in joined.bytes() {
            h ^= byte as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    /// Shared references to circuit breakers (for health loop + remediation).
    pub fn breakers(&self) -> &[Arc<dyn HealthGate>] {
        &self.breakers
    }

    /// Return health per Dog: circuit breaker state + backend status.
    /// Combines CB (execution protection) with Dog::health() (backend observation).
    /// Read cached circuit breaker state — O(1), no network calls.
    /// The health loop updates breakers every 30s via probes.
    /// This just reads the current state from memory.
    pub fn dog_health(&self) -> Vec<(String, String, u32)> {
        self.dogs
            .iter()
            .zip(self.breakers.iter())
            .map(|(dog, cb)| {
                let state = if cb.is_open() {
                    "critical".to_string()
                } else {
                    cb.state()
                };
                (dog.id().to_string(), state, cb.consecutive_failures())
            })
            .collect()
    }

    /// Snapshot of organ quality data for each Dog with an organ handle.
    /// Reads BackendHandle locks briefly — no async, no hold across .await.
    pub fn dog_quality_snapshot(&self) -> Vec<(String, DogStats)> {
        self.dogs
            .iter()
            .enumerate()
            .filter_map(|(idx, dog)| {
                let handle = self.organ_handles[idx].as_ref()?;
                let stats = handle.stats_snapshot()?;
                Some((dog.id().to_string(), stats))
            })
            .collect()
    }

    /// Evaluate a stimulus through Dogs in parallel, aggregate, produce Verdict.
    /// If `filter` is provided, only use Dogs whose IDs match.
    pub async fn evaluate(
        &self,
        stimulus: &Stimulus,
        filter: Option<&[String]>,
        metrics: &Metrics,
    ) -> Result<Verdict, JudgeError> {
        if self.dogs.is_empty() {
            return Err(JudgeError::NoDogs);
        }

        // Filter Dogs if requested — always include deterministic-dog (free, instant)
        let active_dogs: Vec<_> = match filter {
            Some(ids) => {
                let valid_ids: Vec<&str> = self.dogs.iter().map(|d| d.id()).collect();
                self.dogs
                    .iter()
                    .filter(|d| {
                        d.id() == "deterministic-dog"
                            || ids
                                .iter()
                                .any(|id| id == d.id() && valid_ids.contains(&id.as_str()))
                    })
                    .collect()
            }
            None => self.dogs.iter().collect(),
        };

        if active_dogs.is_empty() {
            return Err(JudgeError::NoDogs);
        }

        // Context routing: estimate tokens and filter Dogs that can't handle the stimulus
        let estimated = estimate_tokens(&stimulus.content)
            + stimulus
                .context
                .as_deref()
                .map(estimate_tokens)
                .unwrap_or(0)
            + 400; // fixed overhead (system prompt + axiom template)

        let context_filtered: Vec<_> = active_dogs.iter()
            .filter(|d| {
                let max = d.max_context();
                if max > 0 && estimated > max {
                    tracing::info!(dog_id = %d.id(), estimated_tokens = estimated, max_context = max, "Dog skipped — context too large");
                    false
                } else {
                    true
                }
            })
            .copied()
            .collect();

        let active_dogs = if context_filtered.is_empty() {
            tracing::warn!(
                estimated_tokens = estimated,
                "no Dog has enough context — using all Dogs anyway"
            );
            active_dogs // fallback: try anyway rather than fail
        } else {
            context_filtered
        };

        // Find breaker indices for active dogs
        let dog_breaker_pairs: Vec<_> = active_dogs
            .iter()
            .filter_map(|dog| {
                let idx = self.dogs.iter().position(|d| d.id() == dog.id())?;
                Some((*dog, &self.breakers[idx]))
            })
            .collect();

        // Per-Dog timeout: from config. Sovereign CPU models need 60s+, cloud APIs use 30s.
        // Aligned with HTTP client timeout in openai.rs (both read BackendConfig.timeout_secs).

        // Parallel evaluation — skip Dogs with open circuit breakers or degraded organ quality
        let futures: Vec<_> = dog_breaker_pairs
            .iter()
            .filter(|(dog, cb)| {
                if !cb.should_allow() {
                    return false;
                }
                let dog_idx = self.dogs.iter().position(|d| d.id() == dog.id());
                if let Some(handle) = dog_idx.and_then(|idx| self.organ_handles[idx].as_ref())
                    && handle.is_quality_degraded()
                {
                    tracing::warn!(dog_id = %dog.id(), "Dog skipped — organ quality degraded");
                    return false;
                }
                true
            })
            .map(|(dog, _)| {
                let id = dog.id().to_string();
                let dog_timeout = std::time::Duration::from_secs(dog.timeout_secs());
                async move {
                    let start = std::time::Instant::now();
                    let result = tokio::time::timeout(dog_timeout, dog.evaluate(stimulus)).await;
                    let elapsed_ms = start.elapsed().as_millis() as u64;
                    match result {
                        Ok(inner) => (id, inner, elapsed_ms),
                        Err(_) => (id, Err(DogError::Timeout), elapsed_ms),
                    }
                }
            })
            .collect();

        // Wall-clock timeout: max Dog timeout + 5s safety margin.
        let max_dog_timeout = dog_breaker_pairs
            .iter()
            .filter(|(dog, cb)| {
                if !cb.should_allow() {
                    return false;
                }
                let dog_idx = self.dogs.iter().position(|d| d.id() == dog.id());
                if let Some(handle) = dog_idx.and_then(|idx| self.organ_handles[idx].as_ref())
                    && handle.is_quality_degraded()
                {
                    return false;
                }
                true
            })
            .map(|(dog, _)| dog.timeout_secs())
            .max()
            .unwrap_or(30);
        let wall_clock = std::time::Duration::from_secs(max_dog_timeout + 5);
        let results = tokio::time::timeout(
            wall_clock,
            futures_util::future::join_all(futures),
        ).await.map_err(|elapsed| {
            tracing::error!(wall_clock_secs = max_dog_timeout + 5, %elapsed, "Dog evaluation wall-clock TIMEOUT");
            JudgeError::AllDogsFailed(vec![format!("Evaluation timeout ({elapsed})")])
        })?;

        let mut dog_scores: Vec<DogScore> = Vec::new();
        let mut errors: Vec<String> = Vec::new();
        let mut failed_dogs: Vec<String> = Vec::new();

        for (id, result, elapsed_ms) in results {
            // Find the circuit breaker and organ handle for this dog (by index).
            let dog_idx = self.dogs.iter().position(|d| d.id() == id);
            let cb = dog_idx.map(|idx| &self.breakers[idx]);
            let organ_handle = dog_idx.and_then(|idx| self.organ_handles[idx].as_ref());

            // Count ALL attempts (success + failure) so failure rate = fails / evals ≤ 1.0
            metrics.inc_dog_eval();
            match result {
                Ok(scores) => {
                    if let Some(cb) = cb {
                        cb.record_success();
                    }
                    if let Some(h) = organ_handle {
                        InferenceOrgan::update_stats_entry(h, ScoreOutcome::Success { elapsed_ms });
                    }
                    tracing::info!(
                        phase = "dog_eval", dog_id = %id, latency_ms = elapsed_ms,
                        q = %format!("{:.3}", compute_qscore(&scores).total),
                        raw_fidelity = %format!("{:.3}", scores.fidelity),
                        raw_phi = %format!("{:.3}", scores.phi),
                        raw_verify = %format!("{:.3}", scores.verify),
                        raw_culture = %format!("{:.3}", scores.culture),
                        raw_burn = %format!("{:.3}", scores.burn),
                        raw_sovereignty = %format!("{:.3}", scores.sovereignty),
                        "Dog responded"
                    );
                    dog_scores.push(DogScore {
                        dog_id: id,
                        latency_ms: elapsed_ms,
                        prompt_tokens: scores.prompt_tokens,
                        completion_tokens: scores.completion_tokens,
                        fidelity: phi_bound(scores.fidelity),
                        phi: phi_bound(scores.phi),
                        verify: phi_bound(scores.verify),
                        culture: phi_bound(scores.culture),
                        burn: phi_bound(scores.burn),
                        sovereignty: phi_bound(scores.sovereignty),
                        raw_fidelity: scores.fidelity,
                        raw_phi: scores.phi,
                        raw_verify: scores.verify,
                        raw_culture: scores.culture,
                        raw_burn: scores.burn,
                        raw_sovereignty: scores.sovereignty,
                        abstentions: scores.abstentions,
                        reasoning: scores.reasoning,
                    });
                }
                Err(ref e) => {
                    // ApiError = backend unreachable/protocol error. Trips CB.
                    // ParseError = backend returned garbage (e.g., HTML, malformed JSON).
                    //   Repeated parse errors (3+) trip CB — stops wasting time/money.
                    // RateLimited/Timeout = backend is alive, just congested. Does NOT trip CB.
                    if let Some(cb) = cb {
                        match e {
                            DogError::ApiError(_) | DogError::ParseError(_) => cb.record_failure(),
                            _ => cb.record_success(),
                        }
                    }
                    // Organ: classify failure kind for health tracking.
                    if let Some(h) = organ_handle {
                        let outcome = match e {
                            DogError::ParseError(msg) if msg.contains("zero flood") => {
                                ScoreOutcome::Failure(ScoreFailureKind::ZeroFlood)
                            }
                            DogError::ParseError(msg) if msg.contains("degenerate scores") => {
                                ScoreOutcome::Failure(ScoreFailureKind::Collapse)
                            }
                            DogError::ParseError(_) => {
                                ScoreOutcome::Failure(ScoreFailureKind::ParseError)
                            }
                            DogError::Timeout => ScoreOutcome::Failure(ScoreFailureKind::Timeout),
                            _ => ScoreOutcome::Failure(ScoreFailureKind::ParseError),
                        };
                        InferenceOrgan::update_stats_entry(h, outcome);
                    }
                    metrics.inc_dog_failure();
                    tracing::warn!(
                        phase = "dog_eval", dog_id = %id, latency_ms = elapsed_ms,
                        error_type = %match e {
                            DogError::ApiError(_) => "api_error",
                            DogError::ParseError(_) => "parse_error",
                            DogError::RateLimited(_) => "rate_limited",
                            DogError::Timeout => "timeout",
                        },
                        error = %e,
                        "Dog failed"
                    );
                    failed_dogs.push(id.clone());
                    errors.push(format!("{id}: {e}"));
                }
            }
        }

        if dog_scores.is_empty() {
            return Err(JudgeError::AllDogsFailed(errors));
        }

        // Aggregate: trimmed mean per axiom (remove highest + lowest when >= 4 dogs,
        // otherwise arithmetic mean). This rejects outlier LLM scores that would
        // pollute the consensus — standard robust aggregation (Olympic scoring).
        let avg_fidelity = trimmed_mean(&dog_scores, "fidelity", |s| s.fidelity);
        let avg_phi = trimmed_mean(&dog_scores, "phi", |s| s.phi);
        let avg_verify = trimmed_mean(&dog_scores, "verify", |s| s.verify);
        let avg_culture = trimmed_mean(&dog_scores, "culture", |s| s.culture);
        let avg_burn = trimmed_mean(&dog_scores, "burn", |s| s.burn);
        let avg_sovereignty = trimmed_mean(&dog_scores, "sovereignty", |s| s.sovereignty);

        // Use median Dog's reasoning (deterministic under parallel execution)
        let mut sorted_by_q: Vec<&DogScore> = dog_scores.iter().collect();
        sorted_by_q.sort_by(|a, b| {
            let qa = compute_qscore(&AxiomScores {
                fidelity: a.fidelity,
                phi: a.phi,
                verify: a.verify,
                culture: a.culture,
                burn: a.burn,
                sovereignty: a.sovereignty,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            })
            .total;
            let qb = compute_qscore(&AxiomScores {
                fidelity: b.fidelity,
                phi: b.phi,
                verify: b.verify,
                culture: b.culture,
                burn: b.burn,
                sovereignty: b.sovereignty,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            })
            .total;
            qa.partial_cmp(&qb).unwrap_or(std::cmp::Ordering::Equal)
        });
        let median_reasoning = sorted_by_q
            .get(sorted_by_q.len() / 2)
            .map(|s| s.reasoning.clone())
            .unwrap_or_default();

        let aggregated = AxiomScores {
            fidelity: avg_fidelity,
            phi: avg_phi,
            verify: avg_verify,
            culture: avg_culture,
            burn: avg_burn,
            sovereignty: avg_sovereignty,
            reasoning: median_reasoning,
            ..Default::default()
        };

        let q_score = compute_qscore(&aggregated);
        let kind = verdict_kind(q_score.total);

        // Residual detection: find max per-axiom spread across Dogs
        // This catches cases where Dogs agree on Q-Score total but disagree on individual axioms
        let axiom_names = [
            "fidelity",
            "phi",
            "verify",
            "culture",
            "burn",
            "sovereignty",
        ];
        let (max_disagreement, anomaly_axiom) = if dog_scores.len() > 1 {
            let spreads: Vec<(f64, &str)> = axiom_names
                .iter()
                .map(|&name| {
                    // Exclude Dogs that abstained on this axiom — abstention ≠ disagreement.
                    let values: Vec<f64> = dog_scores
                        .iter()
                        .filter(|s| !s.abstentions.iter().any(|a| a == name))
                        .map(|s| match name {
                            "fidelity" => s.fidelity,
                            "phi" => s.phi,
                            "verify" => s.verify,
                            "culture" => s.culture,
                            "burn" => s.burn,
                            "sovereignty" => s.sovereignty,
                            _ => 0.0,
                        })
                        .collect();
                    if values.len() < 2 {
                        return (0.0, name); // Can't compute spread with < 2 active scores
                    }
                    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
                    (max - min, name)
                })
                .collect();
            let (max_spread, max_axiom) = spreads
                .iter()
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(s, n)| (*s, *n))
                .unwrap_or((0.0, ""));
            if max_spread > PHI_INV2 {
                (max_spread, Some(max_axiom.to_string()))
            } else {
                (max_spread, None)
            }
        } else {
            (0.0, None)
        };
        let anomaly_detected = anomaly_axiom.is_some();

        let mut dog_ids: Vec<&str> = dog_scores.iter().map(|s| s.dog_id.as_str()).collect();
        dog_ids.sort_unstable(); // Canonical order — deterministic regardless of response timing

        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();
        let stimulus_summary: String = stimulus.content.chars().take(100).collect();

        // L1 integrity: compute BLAKE3 hash chained to previous verdict.
        // Single lock acquisition prevents TOCTOU race under concurrent /judge requests.
        let (hash, prev_hash) = {
            let mut lock = self.last_hash.lock().unwrap_or_else(|e| e.into_inner());
            let prev = lock.clone();
            let h = verdict_hash(
                &id,
                q_score.total,
                [
                    q_score.fidelity,
                    q_score.phi,
                    q_score.verify,
                    q_score.culture,
                    q_score.burn,
                    q_score.sovereignty,
                ],
                &stimulus_summary,
                &timestamp,
                prev.as_deref(),
            );
            *lock = Some(h.clone());
            (h, prev)
        };

        let voter_count = dog_scores.len();
        let domain = stimulus.domain.as_deref().unwrap_or("general").to_string();
        Ok(Verdict {
            id,
            domain,
            kind,
            q_score,
            reasoning: aggregated.reasoning,
            dog_id: dog_ids.join("+"),
            stimulus_summary,
            timestamp,
            voter_count,
            dog_scores,
            anomaly_detected,
            max_disagreement,
            anomaly_axiom,
            failed_dogs,
            integrity_hash: Some(hash),
            prev_hash,
        })
    }
}

/// Trimmed mean: drop highest + lowest value when >= 4 scores, average the rest.
/// With 2-3 scores: plain arithmetic mean. Robust against outlier LLM responses.
/// Dogs that abstained on the target axiom are excluded — abstention ≠ low score.
fn trimmed_mean(scores: &[DogScore], axiom_name: &str, extract: impl Fn(&DogScore) -> f64) -> f64 {
    let mut values: Vec<f64> = scores
        .iter()
        .filter(|s| !s.abstentions.iter().any(|a| a == axiom_name))
        .map(&extract)
        .collect();

    // Fallback: if all dogs abstained, include everyone (better than empty)
    if values.is_empty() {
        values = scores.iter().map(&extract).collect();
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if values.len() >= 4 {
        let trimmed = &values[1..values.len() - 1];
        trimmed.iter().sum::<f64>() / trimmed.len() as f64
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// BLAKE3 integrity hash of a verdict — forms a hash chain linking verdicts.
/// Lives here (not in domain/) because blake3 is an external crate.
/// Timestamp is canonicalized to `Z` suffix before hashing — SurrealDB normalizes
/// `+00:00` → `Z` on round-trip, so the hash must be format-independent.
fn verdict_hash(
    id: &str,
    q_total: f64,
    scores: [f64; 6],
    stimulus: &str,
    timestamp: &str,
    prev_hash: Option<&str>,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(id.as_bytes());
    hasher.update(&q_total.to_le_bytes());
    for s in &scores {
        hasher.update(&s.to_le_bytes());
    }
    hasher.update(stimulus.as_bytes());
    // Canonicalize: Rust to_rfc3339() emits "+00:00", SurrealDB returns "Z".
    // Both are semantically identical but byte-different → hash mismatch.
    let canonical_ts = timestamp.replace("+00:00", "Z");
    hasher.update(canonical_ts.as_bytes());
    if let Some(ph) = prev_hash {
        hasher.update(ph.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

/// Verify the BLAKE3 integrity hash of a verdict.
/// Re-derives the hash from verdict fields and compares against the stored hash.
/// Returns `false` if the hash is missing (pre-chain verdict) or mismatches (tampered).
pub fn verify_verdict_integrity(verdict: &Verdict) -> bool {
    let Some(ref stored_hash) = verdict.integrity_hash else {
        return false;
    };
    let recomputed = verdict_hash(
        &verdict.id,
        verdict.q_score.total,
        [
            verdict.q_score.fidelity,
            verdict.q_score.phi,
            verdict.q_score.verify,
            verdict.q_score.culture,
            verdict.q_score.burn,
            verdict.q_score.sovereignty,
        ],
        &verdict.stimulus_summary,
        &verdict.timestamp,
        verdict.prev_hash.as_deref(),
    );
    stored_hash == &recomputed
}

#[derive(Debug)]
pub enum JudgeError {
    NoDogs,
    AllDogsFailed(Vec<String>),
    InvalidInput(String),
}

impl std::fmt::Display for JudgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDogs => write!(f, "No Dogs configured"),
            Self::AllDogsFailed(errs) => write!(f, "All Dogs failed: {}", errs.join("; ")),
            Self::InvalidInput(reason) => write!(f, "Invalid input: {reason}"),
        }
    }
}

impl std::error::Error for JudgeError {}

#[cfg(test)]
mod tests {
    use super::*;

    // A test Dog that returns fixed scores
    struct FixedDog {
        name: String,
        scores: AxiomScores,
    }

    #[async_trait::async_trait]
    impl Dog for FixedDog {
        fn id(&self) -> &str {
            &self.name
        }
        async fn evaluate(&self, _: &Stimulus) -> Result<AxiomScores, DogError> {
            Ok(self.scores.clone())
        }
    }

    struct FailDog;

    #[async_trait::async_trait]
    impl Dog for FailDog {
        fn id(&self) -> &str {
            "fail-dog"
        }
        async fn evaluate(&self, _: &Stimulus) -> Result<AxiomScores, DogError> {
            Err(DogError::ApiError("test failure".into()))
        }
    }

    /// Convenience: build Judge with auto-created CircuitBreaker per Dog (test-only).
    fn test_judge(dogs: Vec<Box<dyn Dog>>) -> Judge {
        let breakers: Vec<Arc<dyn HealthGate>> = dogs
            .iter()
            .map(|d| {
                Arc::new(crate::infra::circuit_breaker::CircuitBreaker::new(
                    d.id().to_string(),
                )) as Arc<dyn HealthGate>
            })
            .collect();
        Judge::new(dogs, breakers)
    }

    fn test_metrics() -> Metrics {
        Metrics::new()
    }

    fn test_stimulus() -> Stimulus {
        Stimulus {
            content: "e4 e5 Nf3".into(),
            context: None,
            domain: Some("chess".into()),
        }
    }

    #[tokio::test]
    async fn single_dog_produces_verdict() {
        let judge = test_judge(vec![Box::new(FixedDog {
            name: "test".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();
        assert!(verdict.q_score.total <= PHI_INV + 1e-10);
        assert!(verdict.q_score.total > 0.0);
        assert_eq!(verdict.dog_id, "test");
        assert_eq!(verdict.dog_scores.len(), 1);
        assert!(!verdict.anomaly_detected);
    }

    #[tokio::test]
    async fn multiple_dogs_averaged() {
        let judge = test_judge(vec![
            Box::new(FixedDog {
                name: "high".into(),
                scores: AxiomScores {
                    fidelity: 0.8,
                    phi: 0.8,
                    verify: 0.8,
                    culture: 0.8,
                    burn: 0.8,
                    sovereignty: 0.8,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
            Box::new(FixedDog {
                name: "low".into(),
                scores: AxiomScores {
                    fidelity: 0.2,
                    phi: 0.2,
                    verify: 0.2,
                    culture: 0.2,
                    burn: 0.2,
                    sovereignty: 0.2,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();
        assert!(verdict.dog_id.contains("high"));
        assert!(verdict.dog_id.contains("low"));
        assert!(verdict.q_score.total > 0.3);
        assert!(verdict.q_score.total < 0.55);
        assert_eq!(verdict.dog_scores.len(), 2);
    }

    #[tokio::test]
    async fn surviving_dog_still_produces_verdict() {
        let judge = test_judge(vec![
            Box::new(FailDog),
            Box::new(FixedDog {
                name: "survivor".into(),
                scores: AxiomScores {
                    fidelity: 0.5,
                    phi: 0.5,
                    verify: 0.5,
                    culture: 0.5,
                    burn: 0.5,
                    sovereignty: 0.5,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();
        assert_eq!(verdict.dog_id, "survivor");
    }

    #[tokio::test]
    async fn all_dogs_fail_returns_error() {
        let judge = test_judge(vec![Box::new(FailDog)]);
        let result = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn no_dogs_returns_error() {
        let judge = test_judge(vec![]);
        let result = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn residual_detection_flags_high_disagreement() {
        let judge = test_judge(vec![
            Box::new(FixedDog {
                name: "optimist".into(),
                scores: AxiomScores {
                    fidelity: 0.9,
                    phi: 0.9,
                    verify: 0.9,
                    culture: 0.9,
                    burn: 0.9,
                    sovereignty: 0.9,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
            Box::new(FixedDog {
                name: "pessimist".into(),
                scores: AxiomScores {
                    fidelity: 0.1,
                    phi: 0.1,
                    verify: 0.1,
                    culture: 0.1,
                    burn: 0.1,
                    sovereignty: 0.1,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();
        assert!(
            verdict.anomaly_detected,
            "Dogs disagreeing 0.9 vs 0.1 should trigger anomaly"
        );
        assert!(verdict.max_disagreement > PHI_INV2);
        assert_eq!(verdict.dog_scores.len(), 2);
        assert!(verdict.anomaly_axiom.is_some());
    }

    #[tokio::test]
    async fn no_anomaly_when_dogs_agree() {
        let judge = test_judge(vec![
            Box::new(FixedDog {
                name: "a".into(),
                scores: AxiomScores {
                    fidelity: 0.5,
                    phi: 0.5,
                    verify: 0.5,
                    culture: 0.5,
                    burn: 0.5,
                    sovereignty: 0.5,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
            Box::new(FixedDog {
                name: "b".into(),
                scores: AxiomScores {
                    fidelity: 0.52,
                    phi: 0.48,
                    verify: 0.51,
                    culture: 0.49,
                    burn: 0.51,
                    sovereignty: 0.50,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();
        assert!(
            !verdict.anomaly_detected,
            "Similar scores should not trigger anomaly"
        );
        assert!(verdict.anomaly_axiom.is_none());
    }

    #[tokio::test]
    async fn filter_selects_specific_dogs() {
        let judge = test_judge(vec![
            Box::new(FixedDog {
                name: "alpha".into(),
                scores: AxiomScores {
                    fidelity: 0.9,
                    phi: 0.9,
                    verify: 0.9,
                    culture: 0.9,
                    burn: 0.9,
                    sovereignty: 0.9,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
            Box::new(FixedDog {
                name: "beta".into(),
                scores: AxiomScores {
                    fidelity: 0.1,
                    phi: 0.1,
                    verify: 0.1,
                    culture: 0.1,
                    burn: 0.1,
                    sovereignty: 0.1,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        let filter = vec!["alpha".to_string()];
        let verdict = judge
            .evaluate(&test_stimulus(), Some(&filter), &test_metrics())
            .await
            .unwrap();
        assert_eq!(verdict.dog_id, "alpha");
        assert_eq!(verdict.dog_scores.len(), 1);
    }

    #[tokio::test]
    async fn filter_with_unknown_id_returns_error() {
        let judge = test_judge(vec![Box::new(FixedDog {
            name: "alpha".into(),
            scores: AxiomScores::default(),
        })]);

        let filter = vec!["nonexistent".to_string()];
        let result = judge
            .evaluate(&test_stimulus(), Some(&filter), &test_metrics())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stimulus_summary_truncated_at_100_chars() {
        let judge = test_judge(vec![Box::new(FixedDog {
            name: "t".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let long_stimulus = Stimulus {
            content: "a".repeat(200),
            context: None,
            domain: None,
        };
        let verdict = judge
            .evaluate(&long_stimulus, None, &test_metrics())
            .await
            .unwrap();
        assert_eq!(verdict.stimulus_summary.len(), 100);
    }

    #[tokio::test]
    async fn circuit_breaker_skips_open_dog() {
        let judge = test_judge(vec![
            Box::new(FailDog),
            Box::new(FixedDog {
                name: "healthy".into(),
                scores: AxiomScores {
                    fidelity: 0.5,
                    phi: 0.5,
                    verify: 0.5,
                    culture: 0.5,
                    burn: 0.5,
                    sovereignty: 0.5,
                    reasoning: AxiomReasoning::default(),
                    ..Default::default()
                },
            }),
        ]);

        // Trip the circuit breaker on FailDog: 3 failures → Open
        for _ in 0..3 {
            let _ = judge
                .evaluate(&test_stimulus(), None, &test_metrics())
                .await;
        }

        // Now FailDog's circuit should be open — only healthy responds
        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();
        assert_eq!(
            verdict.dog_id, "healthy",
            "Open circuit should skip FailDog"
        );
        assert_eq!(verdict.dog_scores.len(), 1);
    }

    #[tokio::test]
    async fn median_reasoning_with_three_dogs() {
        let judge = test_judge(vec![
            Box::new(FixedDog {
                name: "low".into(),
                scores: AxiomScores {
                    fidelity: 0.2,
                    phi: 0.2,
                    verify: 0.2,
                    culture: 0.2,
                    burn: 0.2,
                    sovereignty: 0.2,
                    reasoning: AxiomReasoning {
                        fidelity: "low reasoning".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            }),
            Box::new(FixedDog {
                name: "mid".into(),
                scores: AxiomScores {
                    fidelity: 0.5,
                    phi: 0.5,
                    verify: 0.5,
                    culture: 0.5,
                    burn: 0.5,
                    sovereignty: 0.5,
                    reasoning: AxiomReasoning {
                        fidelity: "mid reasoning".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            }),
            Box::new(FixedDog {
                name: "high".into(),
                scores: AxiomScores {
                    fidelity: 0.8,
                    phi: 0.8,
                    verify: 0.8,
                    culture: 0.8,
                    burn: 0.8,
                    sovereignty: 0.8,
                    reasoning: AxiomReasoning {
                        fidelity: "high reasoning".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            }),
        ]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();
        // Median of 3 sorted by Q → picks index 1 (mid)
        assert_eq!(verdict.reasoning.fidelity, "mid reasoning");
    }

    #[tokio::test]
    async fn dog_health_reports_all_dogs() {
        let judge = test_judge(vec![
            Box::new(FixedDog {
                name: "a".into(),
                scores: AxiomScores::default(),
            }),
            Box::new(FixedDog {
                name: "b".into(),
                scores: AxiomScores::default(),
            }),
        ]);

        let health = judge.dog_health();
        assert_eq!(health.len(), 2);
        assert_eq!(health[0].0, "a");
        assert_eq!(health[0].1, "closed"); // circuit starts closed
        assert_eq!(health[0].2, 0); // zero failures
    }

    #[tokio::test]
    async fn dog_ids_returns_all_names() {
        let judge = test_judge(vec![
            Box::new(FixedDog {
                name: "x".into(),
                scores: AxiomScores::default(),
            }),
            Box::new(FixedDog {
                name: "y".into(),
                scores: AxiomScores::default(),
            }),
        ]);
        let ids = judge.dog_ids();
        assert_eq!(ids, vec!["x", "y"]);
    }

    #[tokio::test]
    async fn verdict_has_valid_uuid_and_timestamp() {
        let judge = test_judge(vec![Box::new(FixedDog {
            name: "t".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();
        // UUID v4 format: 8-4-4-4-12
        assert_eq!(verdict.id.len(), 36);
        assert_eq!(verdict.id.chars().filter(|c| *c == '-').count(), 4);
        // RFC3339 timestamp
        assert!(verdict.timestamp.contains('T'));
        assert!(verdict.timestamp.ends_with("+00:00") || verdict.timestamp.ends_with('Z'));
    }

    // ── trimmed_mean (private fn, direct unit tests) ─────

    fn make_dog_score(id: &str, val: f64) -> DogScore {
        DogScore {
            dog_id: id.into(),
            fidelity: val,
            phi: val,
            verify: val,
            culture: val,
            burn: val,
            sovereignty: val,
            raw_fidelity: val,
            raw_phi: val,
            raw_verify: val,
            raw_culture: val,
            raw_burn: val,
            raw_sovereignty: val,
            ..Default::default()
        }
    }

    #[test]
    fn trimmed_mean_two_values_averages() {
        let scores = vec![make_dog_score("a", 0.2), make_dog_score("b", 0.8)];
        let result = trimmed_mean(&scores, "fidelity", |s| s.fidelity);
        assert!((result - 0.5).abs() < 1e-10);
    }

    #[test]
    fn trimmed_mean_three_values_averages() {
        let scores = vec![
            make_dog_score("a", 0.1),
            make_dog_score("b", 0.5),
            make_dog_score("c", 0.9),
        ];
        let result = trimmed_mean(&scores, "fidelity", |s| s.fidelity);
        assert!((result - 0.5).abs() < 1e-10);
    }

    #[test]
    fn trimmed_mean_four_values_trims_outliers() {
        // With 4+ scores, drop min and max, average the rest
        let scores = vec![
            make_dog_score("outlier_low", 0.0),
            make_dog_score("mid_a", 0.5),
            make_dog_score("mid_b", 0.6),
            make_dog_score("outlier_high", 1.0),
        ];
        let result = trimmed_mean(&scores, "fidelity", |s| s.fidelity);
        // Should be mean of [0.5, 0.6] = 0.55
        assert!((result - 0.55).abs() < 1e-10);
    }

    #[test]
    fn trimmed_mean_five_values_trims_extremes() {
        let scores = vec![
            make_dog_score("a", 0.1),
            make_dog_score("b", 0.4),
            make_dog_score("c", 0.5),
            make_dog_score("d", 0.6),
            make_dog_score("e", 0.9),
        ];
        let result = trimmed_mean(&scores, "fidelity", |s| s.fidelity);
        // Trimmed: [0.4, 0.5, 0.6] → mean = 0.5
        assert!((result - 0.5).abs() < 1e-10);
    }

    #[test]
    fn trimmed_mean_single_value() {
        let scores = vec![make_dog_score("a", 0.7)];
        let result = trimmed_mean(&scores, "fidelity", |s| s.fidelity);
        assert!((result - 0.7).abs() < 1e-10);
    }

    #[test]
    fn trimmed_mean_per_axiom_extraction() {
        let scores = vec![DogScore {
            dog_id: "x".into(),
            fidelity: 0.9,
            phi: 0.1,
            verify: 0.5,
            culture: 0.3,
            burn: 0.7,
            sovereignty: 0.4,
            raw_fidelity: 0.9,
            raw_phi: 0.1,
            raw_verify: 0.5,
            raw_culture: 0.3,
            raw_burn: 0.7,
            raw_sovereignty: 0.4,
            ..Default::default()
        }];
        assert!((trimmed_mean(&scores, "fidelity", |s| s.fidelity) - 0.9).abs() < 1e-10);
        assert!((trimmed_mean(&scores, "phi", |s| s.phi) - 0.1).abs() < 1e-10);
        assert!((trimmed_mean(&scores, "burn", |s| s.burn) - 0.7).abs() < 1e-10);
    }

    #[test]
    fn trimmed_mean_excludes_abstaining_dog() {
        // DeterministicDog abstains on fidelity — its NEUTRAL score should be excluded
        let mut abstainer = make_dog_score("deterministic", 0.309); // NEUTRAL
        abstainer.abstentions = vec!["fidelity".into()];
        let llm = make_dog_score("llm", 0.7);
        let scores = vec![abstainer, llm];

        // Without exclusion: mean(0.309, 0.7) = 0.5045
        // With exclusion: only LLM score = 0.7
        let result = trimmed_mean(&scores, "fidelity", |s| s.fidelity);
        assert!(
            (result - 0.7).abs() < 1e-10,
            "abstaining dog should be excluded, got {result}"
        );
    }

    #[test]
    fn trimmed_mean_includes_non_abstaining_axiom() {
        // Same dog abstains on fidelity but NOT on burn
        let mut dog = make_dog_score("det", 0.5);
        dog.abstentions = vec!["fidelity".into()];
        let scores = vec![dog, make_dog_score("llm", 0.8)];

        // On burn (no abstention): mean(0.5, 0.8) = 0.65
        let result = trimmed_mean(&scores, "burn", |s| s.burn);
        assert!(
            (result - 0.65).abs() < 1e-10,
            "non-abstaining axiom should include both dogs, got {result}"
        );
    }

    // ── hash chain integrity ─────────────────────────────

    #[tokio::test]
    async fn verdict_chain_links_hashes() {
        let judge = test_judge(vec![Box::new(FixedDog {
            name: "chain".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let v1 = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();
        let v2 = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();

        // v1 has no prev (first in chain)
        assert!(v1.prev_hash.is_none());
        assert!(v1.integrity_hash.is_some());

        // v2's prev_hash == v1's integrity_hash
        assert_eq!(v2.prev_hash, v1.integrity_hash);
        assert!(v2.integrity_hash.is_some());
        assert_ne!(v1.integrity_hash, v2.integrity_hash);
    }

    #[test]
    fn seed_chain_sets_initial_hash() {
        let judge = test_judge(vec![Box::new(FixedDog {
            name: "s".into(),
            scores: AxiomScores::default(),
        })]);
        judge.seed_chain(Some("abc123".into()));
        let lock = judge.last_hash.lock().unwrap();
        assert_eq!(*lock, Some("abc123".into()));
    }

    // ── verdict integrity verification ──────────────────────

    #[tokio::test]
    async fn verify_integrity_passes_for_valid_verdict() {
        let judge = test_judge(vec![Box::new(FixedDog {
            name: "v".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();
        assert!(
            verify_verdict_integrity(&verdict),
            "freshly created verdict must pass integrity check"
        );
    }

    #[tokio::test]
    async fn verify_integrity_fails_on_tampered_score() {
        let judge = test_judge(vec![Box::new(FixedDog {
            name: "v".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let mut verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();
        // Tamper: modify a score after hash was computed
        verdict.q_score.fidelity = 0.999;
        assert!(
            !verify_verdict_integrity(&verdict),
            "tampered score must fail integrity check"
        );
    }

    #[tokio::test]
    async fn verify_integrity_fails_on_tampered_summary() {
        let judge = test_judge(vec![Box::new(FixedDog {
            name: "v".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let mut verdict = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();
        verdict.stimulus_summary = "TAMPERED".into();
        assert!(
            !verify_verdict_integrity(&verdict),
            "tampered stimulus must fail integrity check"
        );
    }

    #[test]
    fn verify_integrity_fails_when_hash_missing() {
        let verdict = Verdict {
            id: "test".into(),
            domain: "test".into(),
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
            dog_id: "test".into(),
            stimulus_summary: "test".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            dog_scores: vec![],
            anomaly_detected: false,
            max_disagreement: 0.0,
            anomaly_axiom: None,
            voter_count: 1,
            failed_dogs: vec![],
            integrity_hash: None, // no hash
            prev_hash: None,
        };
        assert!(
            !verify_verdict_integrity(&verdict),
            "verdict without hash must fail verification"
        );
    }

    #[tokio::test]
    async fn verify_integrity_across_chain() {
        let judge = test_judge(vec![Box::new(FixedDog {
            name: "chain".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })]);

        let v1 = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();
        let v2 = judge
            .evaluate(&test_stimulus(), None, &test_metrics())
            .await
            .unwrap();

        // Both verdicts in chain verify independently
        assert!(verify_verdict_integrity(&v1));
        assert!(verify_verdict_integrity(&v2));
        // Chain linkage holds
        assert_eq!(v2.prev_hash, v1.integrity_hash);
    }

    #[tokio::test]
    async fn quality_degraded_dog_is_skipped() {
        let good_dog = FixedDog {
            name: "good".into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        };
        let bad_dog = FixedDog {
            name: "bad".into(),
            scores: AxiomScores {
                fidelity: 0.1,
                phi: 0.1,
                verify: 0.1,
                culture: 0.1,
                burn: 0.1,
                sovereignty: 0.1,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        };

        let mut organ = crate::organ::InferenceOrgan::boot_empty();
        let handle = organ.register_backend(crate::organ::registry::Backend {
            id: crate::organ::registry::BackendId("bad".into()),
            declared: crate::organ::registry::DeclaredCapabilities::default(),
            measured: crate::organ::registry::MeasuredCapabilities::default(),
            health: crate::organ::registry::BackendHealth::Degraded {
                reason: "test quality gate".into(),
                since: std::time::Instant::now(),
            },
        });

        let judge = test_judge(vec![Box::new(good_dog), Box::new(bad_dog)])
            .with_organ_handles(vec![None, Some(handle)]);

        let metrics = Arc::new(test_metrics());
        let verdict = judge
            .evaluate(&test_stimulus(), None, &metrics)
            .await
            .unwrap();

        // bad dog is quality-degraded → should be skipped, only good dog scored
        assert_eq!(verdict.dog_scores.len(), 1);
        assert_eq!(verdict.dog_scores[0].dog_id, "good");
    }
}
