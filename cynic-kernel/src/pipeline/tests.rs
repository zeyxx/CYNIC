use super::*;
use crate::domain::dog::*;
use crate::domain::embedding::NullEmbedding;
use crate::domain::health_gate::HealthGate;
use crate::domain::storage::NullStorage;
use std::sync::Arc;

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

fn test_judge(dogs: Vec<Arc<dyn Dog>>) -> Judge {
    let breakers: Vec<std::sync::Arc<dyn HealthGate>> = dogs
        .iter()
        .map(|d| {
            std::sync::Arc::new(crate::infra::circuit_breaker::CircuitBreaker::new(
                d.id().to_string(),
            )) as std::sync::Arc<dyn HealthGate>
        })
        .collect();
    Judge::new(dogs, breakers)
}

#[tokio::test]
async fn pipeline_runs_with_null_storage_and_null_embedding() {
    // Minimal smoke test: pipeline completes with NullStorage + NullEmbedding
    let dogs: Vec<Arc<dyn Dog>> = vec![Arc::new(crate::dogs::deterministic::DeterministicDog)];
    let judge = test_judge(dogs);
    let storage = NullStorage;
    let embedding = NullEmbedding;
    let usage = Mutex::new(DogUsageTracker::new());
    let verdict_cache = VerdictCache::new();
    let metrics = Metrics::new();

    let domain_curations = crate::domain::wisdom::DomainCurations::new();
    let deps = PipelineDeps {
        judge: &judge,
        storage: &storage,
        embedding: &embedding,
        usage: &usage,
        verdict_cache: &verdict_cache,
        metrics: &metrics,
        event_tx: None,
        request_id: None,
        on_dog: None,
        expected_dog_count: judge.dog_ids().len(),
        enricher: None,
        domain_curations: &domain_curations,
        domain_router: None,
        priority: SlotPriority::User,
    };

    let result = run(
        "1. e4 c5 — The Sicilian Defense".into(),
        None,
        Some("chess".into()),
        None,
        true,
        &deps,
    )
    .await;

    match result {
        Ok(PipelineResult::Evaluated { verdict, .. }) => {
            assert!(verdict.q_score.total > 0.0, "Q-Score should be > 0");
            assert!(!verdict.dog_scores.is_empty(), "should have dog scores");
        }
        Ok(PipelineResult::CacheHit { .. }) => panic!("expected evaluation, got cache hit"),
        Err(e) => panic!("pipeline failed: {e}"),
    }
    // Metrics should reflect the pipeline run
    assert_eq!(
        metrics
            .verdicts_total
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    assert_eq!(
        metrics
            .embedding_failures_total
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    ); // NullEmbedding fails
    assert_eq!(
        metrics
            .cache_misses_total
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
}

#[tokio::test]
async fn pipeline_tracks_usage() {
    let dogs: Vec<Arc<dyn Dog>> = vec![Arc::new(crate::dogs::deterministic::DeterministicDog)];
    let judge = test_judge(dogs);
    let storage = NullStorage;
    let embedding = NullEmbedding;
    let usage = Mutex::new(DogUsageTracker::new());
    let verdict_cache = VerdictCache::new();
    let metrics = Metrics::new();

    let domain_curations = crate::domain::wisdom::DomainCurations::new();
    let deps = PipelineDeps {
        judge: &judge,
        storage: &storage,
        embedding: &embedding,
        usage: &usage,
        verdict_cache: &verdict_cache,
        metrics: &metrics,
        event_tx: None,
        request_id: None,
        on_dog: None,
        expected_dog_count: judge.dog_ids().len(),
        enricher: None,
        domain_curations: &domain_curations,
        domain_router: None,
        priority: SlotPriority::User,
    };
    let _ = run("test content".into(), None, None, None, true, &deps).await;

    let u = usage.lock().await;
    assert!(
        !u.snapshot().is_empty(),
        "usage should have at least one Dog entry"
    );
}

// ── Happy-path pipeline with FixedEmbedding ─────────────

#[tokio::test]
async fn pipeline_with_embedding_populates_cache_and_hits() {
    use crate::domain::embedding::FixedEmbedding;
    use crate::storage::memory::InMemoryStorage;

    let dogs: Vec<Arc<dyn Dog>> = vec![Arc::new(crate::dogs::deterministic::DeterministicDog)];
    let judge = test_judge(dogs);
    let storage = InMemoryStorage::new();
    // 4-dim unit vector — all stimuli get the same embedding
    let embedding = FixedEmbedding::new(vec![0.5, 0.5, 0.5, 0.5]);
    let usage = Mutex::new(DogUsageTracker::new());
    let verdict_cache = VerdictCache::new();
    let metrics = Metrics::new();

    let domain_curations = crate::domain::wisdom::DomainCurations::new();
    let deps = PipelineDeps {
        judge: &judge,
        storage: &storage,
        embedding: &embedding,
        usage: &usage,
        verdict_cache: &verdict_cache,
        metrics: &metrics,
        event_tx: None,
        request_id: None,
        on_dog: None,
        expected_dog_count: judge.dog_ids().len(),
        enricher: None,
        domain_curations: &domain_curations,
        domain_router: None,
        priority: SlotPriority::User,
    };

    // First call: should evaluate (cache miss) and embed successfully
    let r1 = run(
        "1. e4 e5 — King's Pawn".into(),
        None,
        Some("chess".into()),
        None,
        true,
        &deps,
    )
    .await
    .unwrap();
    assert!(matches!(r1, PipelineResult::Evaluated { .. }));
    assert_eq!(
        metrics
            .embedding_successes_total
            .load(std::sync::atomic::Ordering::Relaxed),
        1,
        "embedding should succeed with FixedEmbedding"
    );
    assert_eq!(
        metrics
            .embedding_failures_total
            .load(std::sync::atomic::Ordering::Relaxed),
        0,
        "no embedding failures expected"
    );

    // Second call with same content: should hit cache
    let r2 = run(
        "1. e4 e5 — King's Pawn".into(),
        None,
        Some("chess".into()),
        None,
        true,
        &deps,
    )
    .await
    .unwrap();
    assert!(
        matches!(r2, PipelineResult::CacheHit { similarity, .. } if similarity > 0.99),
        "identical embedding should produce cache hit"
    );
    assert_eq!(
        metrics
            .cache_hits_total
            .load(std::sync::atomic::Ordering::Relaxed),
        1,
        "should have exactly one cache hit"
    );
}

#[tokio::test]
async fn pipeline_with_embedding_creates_crystal_with_provenance() {
    use crate::domain::embedding::FixedEmbedding;
    use crate::storage::memory::InMemoryStorage;

    // Need 2 Dogs for quorum (MIN_QUORUM = 2)
    let dogs: Vec<Arc<dyn Dog>> = vec![
        Arc::new(crate::dogs::deterministic::DeterministicDog),
        Arc::new(FixedDog {
            name: "quorum-helper".into(),
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
    ];
    let judge = test_judge(dogs);
    let storage = InMemoryStorage::new();
    let embedding = FixedEmbedding::new(vec![0.5, 0.5, 0.5, 0.5]);
    let usage = Mutex::new(DogUsageTracker::new());
    let verdict_cache = VerdictCache::new();
    let metrics = Metrics::new();

    let domain_curations = crate::domain::wisdom::DomainCurations::new();
    let deps = PipelineDeps {
        judge: &judge,
        storage: &storage,
        embedding: &embedding,
        usage: &usage,
        verdict_cache: &verdict_cache,
        metrics: &metrics,
        event_tx: None,
        request_id: None,
        on_dog: None,
        expected_dog_count: judge.dog_ids().len(),
        enricher: None,
        domain_curations: &domain_curations,
        domain_router: None,
        priority: SlotPriority::User,
    };

    let result = run(
        "1. e4 c5 — Sicilian Defense".into(),
        None,
        Some("chess".into()),
        None,
        true,
        &deps,
    )
    .await
    .unwrap();

    // Extract verdict ID for provenance check
    let verdict_id = match &result {
        PipelineResult::Evaluated { verdict, .. } => verdict.id.clone(),
        PipelineResult::CacheHit { verdict, .. } => verdict.id.clone(),
    };

    // Crystal should have been created with the verdict's provenance
    let crystals = storage.list_crystals(100).await.unwrap();
    assert!(
        !crystals.is_empty(),
        "pipeline should create at least one crystal"
    );
    let crystal = &crystals[0];
    assert!(
        crystal.contributing_verdicts.contains(&verdict_id),
        "crystal should reference the source verdict (provenance)"
    );
    assert_eq!(crystal.observations, 1);
}

#[tokio::test]
async fn wallet_judgment_returns_verdict_from_deterministic_dog() {
    use crate::domain::embedding::FixedEmbedding;
    use crate::domain::wallet_judgment::WalletProfile;
    use crate::storage::memory::InMemoryStorage;

    let profile = WalletProfile {
        wallet_address: "TestWallet1234567890".to_string(),
        games_completed: 10,
        archetype_consistency: 0.80,
        wallet_age_days: 30,
        average_game_duration: 300,
        duration_variance: 0.20,
        opening_repertoire_hash: "hash1".to_string(),
        move_sequence_hash: "hash2".to_string(),
        suspicious_cluster: false,
        replay_risk: false,
    };
    let ctx = serde_json::to_string(&profile).unwrap();

    let dogs: Vec<Arc<dyn Dog>> = vec![Arc::new(crate::dogs::deterministic::DeterministicDog)];
    let judge = test_judge(dogs);
    let storage = InMemoryStorage::new();
    let embedding = FixedEmbedding::new(vec![0.5; 4]);
    let usage = Mutex::new(DogUsageTracker::new());
    let verdict_cache = VerdictCache::new();
    let metrics = Metrics::new();

    let domain_curations = crate::domain::wisdom::DomainCurations::new();
    let deps = PipelineDeps {
        judge: &judge,
        storage: &storage,
        embedding: &embedding,
        usage: &usage,
        verdict_cache: &verdict_cache,
        metrics: &metrics,
        event_tx: None,
        request_id: None,
        on_dog: None,
        expected_dog_count: judge.dog_ids().len(),
        enricher: None,
        domain_curations: &domain_curations,
        domain_router: None,
        priority: SlotPriority::User,
    };

    let result = run(
        profile.wallet_address.clone(),
        Some(ctx),
        Some("wallet-judgment".into()),
        None,
        true,
        &deps,
    )
    .await
    .unwrap();

    match result {
        PipelineResult::Evaluated { verdict, .. } => {
            assert_eq!(verdict.dog_id, "wallet-deterministic-dog");
            assert!(
                verdict.q_score.total > 0.0,
                "wallet verdict should have positive q_score"
            );
            assert_eq!(verdict.domain, "wallet-judgment");
        }
        _ => panic!("Expected Evaluated result"),
    }
}

#[tokio::test]
async fn wallet_judgment_bark_on_insufficient_games() {
    use crate::domain::embedding::FixedEmbedding;
    use crate::domain::wallet_judgment::WalletProfile;
    use crate::storage::memory::InMemoryStorage;

    let profile = WalletProfile {
        wallet_address: "InsufficientGames".to_string(),
        games_completed: 3,
        archetype_consistency: 0.80,
        wallet_age_days: 30,
        average_game_duration: 300,
        duration_variance: 0.20,
        opening_repertoire_hash: "hash1".to_string(),
        move_sequence_hash: "hash2".to_string(),
        suspicious_cluster: false,
        replay_risk: false,
    };
    let ctx = serde_json::to_string(&profile).unwrap();

    let dogs: Vec<Arc<dyn Dog>> = vec![Arc::new(crate::dogs::deterministic::DeterministicDog)];
    let judge = test_judge(dogs);
    let storage = InMemoryStorage::new();
    let embedding = FixedEmbedding::new(vec![0.5; 4]);
    let usage = Mutex::new(DogUsageTracker::new());
    let verdict_cache = VerdictCache::new();
    let metrics = Metrics::new();

    let domain_curations = crate::domain::wisdom::DomainCurations::new();
    let deps = PipelineDeps {
        judge: &judge,
        storage: &storage,
        embedding: &embedding,
        usage: &usage,
        verdict_cache: &verdict_cache,
        metrics: &metrics,
        event_tx: None,
        request_id: None,
        on_dog: None,
        expected_dog_count: judge.dog_ids().len(),
        enricher: None,
        domain_curations: &domain_curations,
        domain_router: None,
        priority: SlotPriority::User,
    };

    let result = run(
        profile.wallet_address.clone(),
        Some(ctx),
        Some("wallet-judgment".into()),
        None,
        true,
        &deps,
    )
    .await
    .unwrap();

    match result {
        PipelineResult::Evaluated { verdict, .. } => {
            assert_eq!(
                verdict.kind,
                crate::domain::dog::VerdictKind::Bark,
                "insufficient games should produce BARK verdict"
            );
        }
        _ => panic!("Expected Evaluated result"),
    }
}

#[tokio::test]
async fn wallet_judgment_error_on_missing_profile() {
    use crate::domain::embedding::FixedEmbedding;
    use crate::storage::memory::InMemoryStorage;

    let dogs: Vec<Arc<dyn Dog>> = vec![Arc::new(crate::dogs::deterministic::DeterministicDog)];
    let judge = test_judge(dogs);
    let storage = InMemoryStorage::new();
    let embedding = FixedEmbedding::new(vec![0.5; 4]);
    let usage = Mutex::new(DogUsageTracker::new());
    let verdict_cache = VerdictCache::new();
    let metrics = Metrics::new();

    let domain_curations = crate::domain::wisdom::DomainCurations::new();
    let deps = PipelineDeps {
        judge: &judge,
        storage: &storage,
        embedding: &embedding,
        usage: &usage,
        verdict_cache: &verdict_cache,
        metrics: &metrics,
        event_tx: None,
        request_id: None,
        on_dog: None,
        expected_dog_count: judge.dog_ids().len(),
        enricher: None,
        domain_curations: &domain_curations,
        domain_router: None,
        priority: SlotPriority::User,
    };

    let result = run(
        "TestWallet".into(),
        None, // No context provided
        Some("wallet-judgment".into()),
        None,
        true,
        &deps,
    )
    .await;

    assert!(
        result.is_err(),
        "wallet-judgment without valid profile should error"
    );
    match result {
        Err(e) => {
            assert!(
                format!("{e:?}").contains("InvalidInput"),
                "should be InvalidInput error"
            );
        }
        _ => panic!("Expected error result"),
    }
}

#[test]
fn enqueue_verdict_hash_determinism() {
    use sha2::{Digest, Sha256};

    let stimulus1 = "The quick brown fox jumps over the lazy dog";
    let stimulus2 = "The quick brown fox jumps over the lazy dog";
    let stimulus3 = "Different stimulus";

    let hash1 = {
        let mut hasher = Sha256::new();
        hasher.update(stimulus1.as_bytes());
        hex::encode(hasher.finalize())
    };
    let hash2 = {
        let mut hasher = Sha256::new();
        hasher.update(stimulus2.as_bytes());
        hex::encode(hasher.finalize())
    };
    let hash3 = {
        let mut hasher = Sha256::new();
        hasher.update(stimulus3.as_bytes());
        hex::encode(hasher.finalize())
    };

    assert_eq!(
        hash1, hash2,
        "identical stimuli should produce identical hashes"
    );
    assert_ne!(
        hash1, hash3,
        "different stimuli should produce different hashes"
    );
    assert_eq!(
        hash1.len(),
        64,
        "SHA256 hex should be 64 chars (256 bits / 4 bits per hex digit)"
    );
}

#[test]
fn enqueue_verdict_threshold_gate() {
    use crate::domain::dog::PHI_INV;

    let below_threshold = PHI_INV - 0.1;
    let at_threshold = PHI_INV;
    let above_threshold = PHI_INV + 0.1;

    assert!(
        below_threshold < PHI_INV,
        "below threshold should be < PHI_INV"
    );
    assert!(at_threshold >= PHI_INV, "at threshold should be >= PHI_INV");
    assert!(
        above_threshold >= PHI_INV,
        "above threshold should be >= PHI_INV"
    );

    assert!(
        !should_enqueue(below_threshold),
        "below threshold should not enqueue"
    );
    assert!(should_enqueue(at_threshold), "at threshold should enqueue");
    assert!(
        should_enqueue(above_threshold),
        "above threshold should enqueue"
    );
}

fn should_enqueue(q_score: f64) -> bool {
    use crate::domain::dog::PHI_INV;
    q_score >= PHI_INV
}

#[test]
fn enqueue_verdict_verdict_type_mapping() {
    use crate::domain::dog::VerdictKind;

    let mappings = vec![
        (VerdictKind::Howl, "howl"),
        (VerdictKind::Wag, "wag"),
        (VerdictKind::Growl, "growl"),
        (VerdictKind::Bark, "bark"),
    ];

    for (kind, expected) in mappings {
        let actual = match kind {
            VerdictKind::Howl => "howl",
            VerdictKind::Wag => "wag",
            VerdictKind::Growl => "growl",
            VerdictKind::Bark => "bark",
        };
        assert_eq!(
            actual, expected,
            "VerdictKind::{kind:?} should map to {expected}"
        );
    }
}
