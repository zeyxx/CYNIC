//! Judge — orchestrates Dogs, computes consensus, emits Verdicts.
//! Parallel evaluation via FuturesUnordered (progressive Dog arrival).
//! Residual detection: disagreement > φ⁻² = ANOMALY.

mod math;
mod types;

pub use math::verify_verdict_integrity;
pub use types::{DogFailure, DogFailureKind, JudgeError};

use crate::domain::dog::{estimate_tokens, *};
use crate::domain::health_gate::HealthGate;
use crate::domain::metrics::Metrics;
use crate::organ::health::{DogStats, ScoreFailureKind};
use crate::organ::{BackendHandle, InferenceOrgan, ScoreOutcome};
use chrono::Utc;
use futures_util::StreamExt;
#[cfg(test)]
use math::trimmed_mean;
use math::{aggregate_scores, chain_hash, detect_residuals};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct Judge {
    dogs: Vec<Arc<dyn Dog>>,
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

#[derive(Clone, Copy)]
struct RunnableDog<'a> {
    idx: usize,
    dog: &'a Arc<dyn Dog>,
}

impl Judge {
    pub fn new(dogs: Vec<Arc<dyn Dog>>, breakers: Vec<Arc<dyn HealthGate>>) -> Self {
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

    /// Clone internal Arcs for roster reconstruction (refcount++, not deep copy).
    pub fn dogs(&self) -> &[Arc<dyn Dog>] {
        &self.dogs
    }

    /// Clone internal organ handles for roster reconstruction.
    pub fn organ_handles(&self) -> &[Option<BackendHandle>] {
        &self.organ_handles
    }

    /// Snapshot the current chain hash (for preserving across Judge swaps).
    pub fn last_hash_snapshot(&self) -> Option<String> {
        self.last_hash.lock().ok().and_then(|g| g.clone())
    }

    /// Build a new Judge without the named Dog. Returns None if not found.
    /// Clones existing Arcs (refcount++), preserves chain hash.
    pub fn without_dog(current: &Judge, dog_id: &str) -> Option<Judge> {
        let idx = current.dogs.iter().position(|d| d.id() == dog_id)?;
        let mut dogs: Vec<Arc<dyn Dog>> = current.dogs.iter().map(Arc::clone).collect();
        let mut breakers: Vec<Arc<dyn HealthGate>> =
            current.breakers.iter().map(Arc::clone).collect();
        let mut handles: Vec<Option<BackendHandle>> = current.organ_handles.clone();
        dogs.remove(idx);
        breakers.remove(idx);
        handles.remove(idx);
        let new_judge = Judge::new(dogs, breakers).with_organ_handles(handles);
        let chain = current.last_hash_snapshot();
        new_judge.seed_chain(chain);
        Some(new_judge)
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
                .filter(|(d, cb)| cb.should_allow() && f.iter().any(|id| id == d.id()))
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

    fn selected_candidate_indices(
        &self,
        stimulus: &Stimulus,
        filter: Option<&[String]>,
    ) -> Result<Vec<usize>, JudgeError> {
        if self.dogs.is_empty() {
            return Err(JudgeError::NoDogs);
        }

        // Filter Dogs if requested
        let requested_indices: Vec<usize> = match filter {
            Some(ids) => {
                let valid_ids: Vec<&str> = self.dogs.iter().map(|d| d.id()).collect();
                self.dogs
                    .iter()
                    .enumerate()
                    .filter(|d| {
                        ids.iter()
                            .any(|id| id == d.1.id() && valid_ids.contains(&id.as_str()))
                    })
                    .map(|(idx, _)| idx)
                    .collect()
            }
            None => (0..self.dogs.len()).collect(),
        };

        if requested_indices.is_empty() {
            return Err(JudgeError::NoDogs);
        }

        // Context routing: estimate tokens and filter Dogs that can't handle the stimulus.
        let estimated = estimate_tokens(&stimulus.content)
            + stimulus
                .context
                .as_deref()
                .map(estimate_tokens)
                .unwrap_or(0)
            + 400; // fixed overhead (system prompt + axiom template)

        let context_filtered: Vec<usize> = requested_indices
            .iter()
            .copied()
            .filter(|&idx| {
                let dog = &self.dogs[idx];
                let max = dog.max_context();
                if max > 0 && estimated > max {
                    tracing::info!(
                        dog_id = %dog.id(),
                        estimated_tokens = estimated,
                        max_context = max,
                        "Dog skipped — context too large"
                    );
                    false
                } else {
                    true
                }
            })
            .collect();

        Ok(if context_filtered.is_empty() {
            tracing::warn!(
                estimated_tokens = estimated,
                "no Dog has enough context — using all Dogs anyway"
            );
            requested_indices // fallback: try anyway rather than fail
        } else {
            context_filtered
        })
    }

    /// Count Dogs that would actually run after filter, context routing and health gates.
    pub fn runnable_dog_count(
        &self,
        stimulus: &Stimulus,
        filter: Option<&[String]>,
    ) -> Result<usize, JudgeError> {
        let candidate_indices = self.selected_candidate_indices(stimulus, filter)?;
        Ok(self.runnable_dogs(&candidate_indices).len())
    }

    fn runnable_dogs<'a>(&'a self, candidate_indices: &[usize]) -> Vec<RunnableDog<'a>> {
        candidate_indices
            .iter()
            .copied()
            .filter_map(|idx| {
                let dog = &self.dogs[idx];
                let breaker = &self.breakers[idx];
                if !breaker.should_allow() {
                    tracing::warn!(
                        dog_id = %dog.id(),
                        circuit_state = %breaker.state(),
                        "Dog skipped — circuit breaker open"
                    );
                    return None;
                }
                if let Some(handle) = self.organ_handles[idx].as_ref()
                    && handle.is_quality_degraded()
                {
                    if handle.should_allow_quality_probe() {
                        tracing::info!(
                            dog_id = %dog.id(),
                            "quality probe — degradation TTL expired, allowing one evaluation"
                        );
                    } else {
                        tracing::warn!(
                            dog_id = %dog.id(),
                            "Dog skipped — organ quality degraded"
                        );
                        return None;
                    }
                }
                Some(RunnableDog { idx, dog })
            })
            .collect()
    }

    /// Evaluate a stimulus through Dogs in parallel, aggregate, produce Verdict.
    /// If `filter` is provided, only use Dogs whose IDs match.
    #[tracing::instrument(skip(self, metrics), err)]
    pub async fn evaluate(
        &self,
        stimulus: &Stimulus,
        filter: Option<&[String]>,
        metrics: &Metrics,
    ) -> Result<Verdict, JudgeError> {
        self.evaluate_progressive(stimulus, filter, metrics, None)
            .await
    }

    /// Evaluate with optional progressive callback — called after each Dog completes.
    /// `on_dog` receives (dog_id, success, elapsed_ms, optional DogScore clone).
    /// Used by `/judge/async` to update the job store as Dogs arrive.
    #[tracing::instrument(skip(self, metrics), err)]
    fn process_dog_result(
        &self,
        idx: usize,
        id: String,
        result: Result<AxiomScores, DogError>,
        elapsed_ms: u64,
        metrics: &Metrics,
    ) -> Result<DogScore, DogFailure> {
        let cb = &self.breakers[idx];
        let organ_handle = self.organ_handles[idx].as_ref();

        metrics.inc_dog_eval();
        match result {
            Ok(scores) => {
                cb.record_success();
                if let Some(h) = organ_handle {
                    InferenceOrgan::update_stats_entry(
                        h,
                        ScoreOutcome::Success {
                            elapsed_ms,
                            completion_tokens: scores.completion_tokens,
                            thinking_tokens: scores.thinking_tokens,
                        },
                    );
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
                Ok(DogScore {
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
                })
            }
            Err(e) => {
                match &e {
                    DogError::ApiError(_)
                    | DogError::ParseError(_)
                    | DogError::ZeroFlood(_)
                    | DogError::DegenerateScores { .. } => cb.record_failure(),
                    // ContextOverflow is a pre-condition, not a quality signal —
                    // don't penalize the Dog's circuit breaker.
                    DogError::ContextOverflow { .. } => {}
                    _ => cb.record_success(),
                }
                if let Some(h) = organ_handle {
                    let outcome = match &e {
                        DogError::ZeroFlood(_) => {
                            ScoreOutcome::Failure(ScoreFailureKind::ZeroFlood)
                        }
                        DogError::DegenerateScores { .. } => {
                            ScoreOutcome::Failure(ScoreFailureKind::Collapse)
                        }
                        DogError::ParseError(_) => {
                            ScoreOutcome::Failure(ScoreFailureKind::ParseError)
                        }
                        DogError::Timeout => ScoreOutcome::Failure(ScoreFailureKind::Timeout),
                        _ => ScoreOutcome::Failure(ScoreFailureKind::ApiError),
                    };
                    InferenceOrgan::update_stats_entry(h, outcome);
                }
                metrics.inc_dog_failure();
                let kind = DogFailureKind::from(&e);
                tracing::warn!(
                    phase = "dog_eval", dog_id = %id, latency_ms = elapsed_ms,
                    error_type = %kind.as_str(),
                    error = %e,
                    "Dog failed"
                );
                Err(DogFailure {
                    dog_id: id,
                    kind,
                    detail: e.to_string(),
                })
            }
        }
    }

    pub async fn evaluate_progressive(
        &self,
        stimulus: &Stimulus,
        filter: Option<&[String]>,
        metrics: &Metrics,
        on_dog: Option<&crate::pipeline::OnDogCallback>,
    ) -> Result<Verdict, JudgeError> {
        let candidate_indices = self.selected_candidate_indices(stimulus, filter)?;
        let runnable_dogs = self.runnable_dogs(&candidate_indices);

        // Per-Dog timeout: from config. Sovereign CPU models need 60s+, cloud APIs use 30s.
        // Aligned with HTTP client timeout in openai.rs (both read BackendConfig.timeout_secs).

        // Parallel evaluation — skip Dogs with open circuit breakers or degraded organ quality.
        // Uses FuturesUnordered for progressive Dog arrival (D4: /judge/status polling).
        let mut futs = futures_util::stream::FuturesUnordered::new();
        for entry in &runnable_dogs {
            let idx = entry.idx;
            let dog = entry.dog;
            let id = dog.id().to_string();
            let dog_timeout = std::time::Duration::from_secs(dog.timeout_secs());
            futs.push(async move {
                let start = std::time::Instant::now();
                let result = tokio::time::timeout(dog_timeout, dog.evaluate(stimulus)).await;
                let elapsed_ms = start.elapsed().as_millis() as u64;
                match result {
                    Ok(inner) => (idx, id, inner, elapsed_ms),
                    Err(_) => (idx, id, Err(DogError::Timeout), elapsed_ms),
                }
            });
        }

        // Wall-clock timeout: max Dog timeout + 5s safety margin.
        let max_dog_timeout = runnable_dogs
            .iter()
            .map(|entry| entry.dog.timeout_secs())
            .max()
            .unwrap_or(30);
        let wall_clock = std::time::Duration::from_secs(max_dog_timeout + 5);
        let deadline = tokio::time::Instant::now() + wall_clock;

        // O5: Early verdict on quorum arrival
        // Quorum = majority (⌈n/2⌉ for n Dogs). Return as soon as we have it.
        // Prevents waiting for slow Dogs (e.g., qwen35-gpu @8.8s) if 3/5 Dogs are ready @5s.
        let quorum_count = (runnable_dogs.len() / 2) + 1;

        let mut dog_scores: Vec<DogScore> = Vec::new();
        let mut failures: Vec<DogFailure> = Vec::new();
        let mut failed_dogs: Vec<String> = Vec::new();
        let mut failed_dog_errors: std::collections::HashMap<String, String> = Default::default();

        // Process Dogs as they arrive (progressive) instead of waiting for all (join_all).
        // O5: Return early once quorum_count Dogs have completed successfully.
        while let Some(result) = tokio::time::timeout_at(deadline, futs.next()).await.map_err(|elapsed| {
            tracing::error!(wall_clock_secs = max_dog_timeout + 5, %elapsed, "Dog evaluation wall-clock TIMEOUT");
            JudgeError::AllDogsFailed(vec![DogFailure {
                dog_id: "*".into(),
                kind: DogFailureKind::Timeout,
                detail: format!("Wall-clock timeout ({elapsed})"),
            }])
        })? {
            let (idx, id, res, elapsed_ms) = result;
            match self.process_dog_result(idx, id.clone(), res, elapsed_ms, metrics) {
                Ok(score) => {
                    if let Some(cb) = &on_dog {
                        cb(&id, true, elapsed_ms, Some(&score), None);
                    }
                    dog_scores.push(score);

                    // O5: Early exit on quorum. Still include failed Dogs in final verdict,
                    // but return verdict as soon as we have enough successful scorers.
                    if dog_scores.len() >= quorum_count {
                        tracing::info!(
                            phase = "early_verdict",
                            quorum_count = quorum_count,
                            dogs_completed = dog_scores.len(),
                            failed_dogs = failed_dogs.len(),
                            "Quorum reached — returning early verdict"
                        );
                        break;
                    }
                }
                Err(failure) => {
                    if let Some(cb) = &on_dog {
                        cb(&id, false, elapsed_ms, None, Some(failure.detail.clone()));
                    }
                    failed_dog_errors.insert(id.clone(), failure.kind.as_str().to_string());
                    failed_dogs.push(id.clone());
                    failures.push(failure);
                    // Don't exit on failure — keep waiting for more Dogs to reach quorum
                }
            }
        }

        if dog_scores.is_empty() {
            return Err(JudgeError::AllDogsFailed(failures));
        }

        // Aggregate (trimmed mean per axiom + median reasoning) and detect anomalies.
        // Rejects outlier LLM scores (Olympic scoring), catches axiom disagreement.
        let aggregated = aggregate_scores(&dog_scores);
        let q_score = compute_qscore(&aggregated);
        let kind = verdict_kind(q_score.total);
        let (max_disagreement, anomaly_axiom) = detect_residuals(&dog_scores);
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
            let h = chain_hash(
                &id,
                &q_score,
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
            failed_dog_errors,
            integrity_hash: Some(hash),
            prev_hash,
        })
    }
}

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
    fn test_judge(dogs: Vec<Arc<dyn Dog>>) -> Judge {
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
            request_id: None,
        }
    }

    #[tokio::test]
    async fn single_dog_produces_verdict() {
        let judge = test_judge(vec![Arc::new(FixedDog {
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
            Arc::new(FixedDog {
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
            Arc::new(FixedDog {
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
            Arc::new(FailDog),
            Arc::new(FixedDog {
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
        let judge = test_judge(vec![Arc::new(FailDog)]);
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
            Arc::new(FixedDog {
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
            Arc::new(FixedDog {
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
            Arc::new(FixedDog {
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
            Arc::new(FixedDog {
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
            Arc::new(FixedDog {
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
            Arc::new(FixedDog {
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
        let judge = test_judge(vec![Arc::new(FixedDog {
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
        let judge = test_judge(vec![Arc::new(FixedDog {
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
            request_id: None,
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
            Arc::new(FailDog),
            Arc::new(FixedDog {
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
            Arc::new(FixedDog {
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
            Arc::new(FixedDog {
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
            Arc::new(FixedDog {
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
            Arc::new(FixedDog {
                name: "a".into(),
                scores: AxiomScores::default(),
            }),
            Arc::new(FixedDog {
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
            Arc::new(FixedDog {
                name: "x".into(),
                scores: AxiomScores::default(),
            }),
            Arc::new(FixedDog {
                name: "y".into(),
                scores: AxiomScores::default(),
            }),
        ]);
        let ids = judge.dog_ids();
        assert_eq!(ids, vec!["x", "y"]);
    }

    #[tokio::test]
    async fn verdict_has_valid_uuid_and_timestamp() {
        let judge = test_judge(vec![Arc::new(FixedDog {
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
        let judge = test_judge(vec![Arc::new(FixedDog {
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
        let judge = test_judge(vec![Arc::new(FixedDog {
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
        let judge = test_judge(vec![Arc::new(FixedDog {
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
        let judge = test_judge(vec![Arc::new(FixedDog {
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
        let judge = test_judge(vec![Arc::new(FixedDog {
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
            failed_dog_errors: Default::default(),
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
        let judge = test_judge(vec![Arc::new(FixedDog {
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

        let judge = test_judge(vec![Arc::new(good_dog), Arc::new(bad_dog)])
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

#[cfg(test)]
mod roster_tests {
    use super::*;
    use arc_swap::ArcSwap;

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

    fn make_dog(name: &str) -> Arc<dyn Dog> {
        Arc::new(FixedDog {
            name: name.into(),
            scores: AxiomScores {
                fidelity: 0.5,
                phi: 0.4,
                verify: 0.3,
                culture: 0.5,
                burn: 0.4,
                sovereignty: 0.3,
                reasoning: AxiomReasoning::default(),
                ..Default::default()
            },
        })
    }

    fn make_breaker(name: &str) -> Arc<dyn HealthGate> {
        Arc::new(crate::infra::circuit_breaker::CircuitBreaker::new(
            name.into(),
        ))
    }

    #[tokio::test]
    async fn arcswap_roster_swap_preserves_chain_and_adds_dog() {
        // Start with 1 Dog
        let dog_a = make_dog("alpha");
        let breaker_a = make_breaker("alpha");
        let judge = Judge::new(vec![dog_a.clone()], vec![breaker_a.clone()]);
        judge.seed_chain(Some("seed-hash-123".into()));

        // Wrap in ArcSwap (simulates AppState)
        let swap = ArcSwap::from_pointee(judge);

        // Load current, verify 1 Dog
        let current = swap.load_full();
        assert_eq!(current.dog_ids(), vec!["alpha"]);

        // Simulate registration: clone Arcs + add new Dog
        let mut dogs: Vec<Arc<dyn Dog>> = current.dogs().iter().map(Arc::clone).collect();
        let mut breakers: Vec<Arc<dyn HealthGate>> =
            current.breakers().iter().map(Arc::clone).collect();
        let mut handles: Vec<Option<crate::organ::BackendHandle>> =
            current.organ_handles().to_vec();

        dogs.push(make_dog("beta"));
        breakers.push(make_breaker("beta"));
        handles.push(None);

        let new_judge = Judge::new(dogs, breakers).with_organ_handles(handles);
        // Preserve chain
        let chain = current.last_hash_snapshot();
        assert_eq!(chain, Some("seed-hash-123".into()));
        new_judge.seed_chain(chain);

        // Atomic swap
        swap.store(Arc::new(new_judge));

        // Verify new roster
        let updated = swap.load_full();
        let ids = updated.dog_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"alpha".to_string()));
        assert!(ids.contains(&"beta".to_string()));

        // Verify chain preserved
        assert_eq!(updated.last_hash_snapshot(), Some("seed-hash-123".into()));

        // Verify old judge still accessible (concurrent readers)
        assert_eq!(current.dog_ids(), vec!["alpha"]);
    }

    #[tokio::test]
    async fn without_dog_removes_target_and_preserves_chain() {
        let dog_a = make_dog("alpha");
        let dog_b = make_dog("beta");
        let judge = Judge::new(
            vec![dog_a, dog_b],
            vec![make_breaker("alpha"), make_breaker("beta")],
        );
        judge.seed_chain(Some("hash-abc".into()));

        let reduced = Judge::without_dog(&judge, "beta").expect("beta exists");
        assert_eq!(reduced.dog_ids(), vec!["alpha"]);
        assert_eq!(reduced.last_hash_snapshot(), Some("hash-abc".into()));
        assert_eq!(reduced.breakers().len(), 1);
    }

    #[test]
    fn without_dog_returns_none_for_unknown() {
        let judge = Judge::new(vec![make_dog("alpha")], vec![make_breaker("alpha")]);
        assert!(Judge::without_dog(&judge, "nonexistent").is_none());
    }

    #[tokio::test]
    async fn new_dog_evaluates_after_swap() {
        let dog_a = make_dog("alpha");
        let swap = ArcSwap::from_pointee(Judge::new(vec![dog_a], vec![make_breaker("alpha")]));

        // Add beta
        let current = swap.load_full();
        let mut dogs: Vec<Arc<dyn Dog>> = current.dogs().iter().map(Arc::clone).collect();
        let mut breakers: Vec<Arc<dyn HealthGate>> =
            current.breakers().iter().map(Arc::clone).collect();
        dogs.push(make_dog("beta"));
        breakers.push(make_breaker("beta"));
        let new_judge = Judge::new(dogs, breakers);
        swap.store(Arc::new(new_judge));

        // Evaluate with new roster — both Dogs should contribute
        let stimulus = Stimulus {
            content: "test".into(),
            context: None,
            domain: None,
            request_id: None,
        };
        let verdict = swap
            .load_full()
            .evaluate(&stimulus, None, &Metrics::new())
            .await
            .unwrap();
        assert_eq!(verdict.dog_scores.len(), 2);
        assert!(verdict.dog_id.contains("alpha"));
        assert!(verdict.dog_id.contains("beta"));
    }

    #[tokio::test]
    async fn registered_dog_expires_without_heartbeat() {
        let dog_a = make_dog("permanent");
        let dog_b = make_dog("ephemeral");
        let judge = Judge::new(
            vec![dog_a, dog_b],
            vec![make_breaker("permanent"), make_breaker("ephemeral")],
        );

        let swap = ArcSwap::from_pointee(judge);
        assert_eq!(swap.load_full().dog_ids().len(), 2);

        // Simulate TTL expiry: remove ephemeral
        let current = swap.load_full();
        let reduced = Judge::without_dog(&current, "ephemeral").unwrap();
        swap.store(Arc::new(reduced));

        let final_roster = swap.load_full();
        assert_eq!(final_roster.dog_ids(), vec!["permanent"]);
        // Permanent dog still evaluates
        let verdict = final_roster
            .evaluate(
                &Stimulus {
                    content: "test".into(),
                    context: None,
                    domain: None,
                    request_id: None,
                },
                None,
                &Metrics::new(),
            )
            .await
            .unwrap();
        assert_eq!(verdict.dog_scores.len(), 1);
    }

    #[tokio::test]
    async fn early_verdict_quorum_with_five_dogs() {
        // 5 dogs: quorum = (5/2)+1 = 3. Early verdict returns when quorum is reached.
        let dogs = vec![
            make_dog("dog-a"),
            make_dog("dog-b"),
            make_dog("dog-c"),
            make_dog("dog-d"),
            make_dog("dog-e"),
        ];

        let judge = Judge::new(
            dogs,
            vec![
                make_breaker("dog-a"),
                make_breaker("dog-b"),
                make_breaker("dog-c"),
                make_breaker("dog-d"),
                make_breaker("dog-e"),
            ],
        );

        let stimulus = Stimulus {
            content: "test token".into(),
            context: None,
            domain: None,
            request_id: None,
        };

        let verdict = judge
            .evaluate(&stimulus, None, &Metrics::new())
            .await
            .unwrap();

        // Early verdict returns when quorum (3) dogs complete.
        // Verdict should have at least quorum minimum.
        assert!(
            verdict.dog_scores.len() >= 3,
            "expected at least quorum (3) dog scores, got {}",
            verdict.dog_scores.len()
        );
    }
}
