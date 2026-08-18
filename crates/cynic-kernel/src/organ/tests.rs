use super::*;
use crate::domain::dog_health::{DogStats, ScoreFailureKind, ScoreOutcome};
use crate::organ::registry::{
    Backend, BackendHealth, BackendId, DeclaredCapabilities, MeasuredCapabilities,
};
use std::time::{Duration, Instant};

fn make_backend(id: &str) -> Backend {
    Backend {
        id: BackendId(id.to_string()),
        declared: DeclaredCapabilities::default(),
        measured: MeasuredCapabilities::default(),
        health: BackendHealth::Healthy,
    }
}

#[test]
fn organ_starts_empty() {
    let organ = InferenceOrgan::boot_empty();
    assert_eq!(organ.backend_count(), 0);
}

#[test]
fn register_increments_count() {
    let mut organ = InferenceOrgan::boot_empty();
    organ.register_backend(make_backend("dog-a"));
    organ.register_backend(make_backend("dog-b"));
    assert_eq!(organ.backend_count(), 2);
}

#[test]
fn stats_start_pessimistic() {
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("dog-a"));
    let stats = handle.stats_snapshot().unwrap();
    assert_eq!(stats.json_valid_rate(), 0.0); // K14
    assert_eq!(stats.total_calls, 0);
}

#[test]
fn update_success_increments_rate() {
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("dog-a"));
    InferenceOrgan::update_stats_entry(
        &handle,
        ScoreOutcome::Success {
            elapsed_ms: 0,
            completion_tokens: 0,
            thinking_tokens: 0,
        },
    );
    InferenceOrgan::update_stats_entry(
        &handle,
        ScoreOutcome::Success {
            elapsed_ms: 0,
            completion_tokens: 0,
            thinking_tokens: 0,
        },
    );
    InferenceOrgan::update_stats_entry(&handle, ScoreOutcome::Failure(ScoreFailureKind::ZeroFlood));
    let stats = handle.stats_snapshot().unwrap();
    assert_eq!(stats.total_calls, 3);
    assert!((stats.json_valid_rate() - 2.0 / 3.0).abs() < 1e-10);
}

#[test]
fn healthy_backend_is_not_quality_degraded() {
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("dog-a"));
    assert!(!handle.is_quality_degraded());
}

#[test]
fn degraded_backend_is_quality_degraded() {
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("dog-a"));
    // Trip the gate: 6 failures out of 10 (>50%)
    for _ in 0..4 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 0,
                completion_tokens: 0,
                thinking_tokens: 0,
            },
        );
    }
    for _ in 0..6 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::ParseError),
        );
    }
    assert!(handle.is_quality_degraded());
}

#[test]
fn quality_probe_allowed_after_ttl_expired() {
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("latched-dog"));
    // Trip the gate: 10 failures → degraded
    for _ in 0..10 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::ParseError),
        );
    }
    assert!(
        handle.is_quality_degraded(),
        "precondition: must be degraded"
    );

    // Before TTL: no probe allowed
    assert!(
        !handle.should_allow_quality_probe(),
        "probe must be denied before TTL expires"
    );

    // Simulate TTL expiry by backdating the degradation timestamp
    {
        let mut guard = handle.0.lock().unwrap();
        guard.backend.health = BackendHealth::Degraded {
            reason: "parse failure rate 80% > 50% in last 10 calls".into(),
            since: Instant::now() - Duration::from_secs(31),
        };
    }

    // After TTL: first probe allowed
    assert!(
        handle.should_allow_quality_probe(),
        "probe must be allowed after TTL expires"
    );
    // Second probe immediately: denied (rate-limited)
    assert!(
        !handle.should_allow_quality_probe(),
        "second probe within TTL must be denied"
    );
}

#[test]
fn quality_probe_success_recovers_dog() {
    // Full latch-recovery cycle: degrade → TTL expires → probe succeeds → Healthy
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("recovering-latched"));
    // Trip: 10 failures
    for _ in 0..10 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::ParseError),
        );
    }
    assert!(handle.is_quality_degraded());

    // Simulate TTL expiry + clear the gate window with successes
    {
        let mut guard = handle.0.lock().unwrap();
        guard.backend.health = BackendHealth::Degraded {
            reason: "parse failure rate 80% > 50% in last 10 calls".into(),
            since: Instant::now() - Duration::from_secs(31),
        };
    }
    // Probe allowed — simulate what the judge does: evaluate + update_stats_entry
    assert!(handle.should_allow_quality_probe());
    // 10 successes push failures out of the 10-call sliding window
    for _ in 0..10 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 100,
                completion_tokens: 200,
                thinking_tokens: 0,
            },
        );
    }
    // Dog should be healthy now — gate cleared, sync_parse_gate_health promoted
    assert!(
        !handle.is_quality_degraded(),
        "Dog must recover after successful probe clears the gate window"
    );
}

#[test]
fn recovery_promotes_degraded_to_healthy() {
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("dog-a"));
    // Trip the gate: fill window with failures
    for _ in 0..10 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::ParseError),
        );
    }
    assert!(handle.is_quality_degraded());

    // Recover: 10 successes evict all failures from window
    for _ in 0..10 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 0,
                completion_tokens: 0,
                thinking_tokens: 0,
            },
        );
    }
    // Gate is no longer tripped → should recover to Healthy
    assert!(!handle.is_quality_degraded());
}

#[test]
fn gate_trips_and_degrades_backend() {
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("dog-a"));
    // 5 successes + 6 failures → gate trips at 6/11 > 50%
    for _ in 0..5 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 0,
                completion_tokens: 0,
                thinking_tokens: 0,
            },
        );
    }
    for _ in 0..6 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::ZeroFlood),
        );
    }
    let guard = handle.0.lock().unwrap();
    assert!(
        matches!(guard.backend.health, BackendHealth::Degraded { .. }),
        "backend should be degraded after gate trips"
    );
}

#[test]
fn restore_stats_excludes_api_errors_from_gate() {
    // F1 fix: a Dog with 8 api_errors (e.g. 401 from wrong auth) and 0 success
    // must NOT have its gate tripped after restore. ApiError is infra, not model quality.
    let mut organ = InferenceOrgan::boot_empty();
    let _handle = organ.register_backend(make_backend("gemma-4b-core"));

    let stats = DogStats {
        total_calls: 8,
        success_count: 0,
        zero_flood_count: 0,
        collapse_count: 0,
        parse_error_count: 0,
        timeout_count: 0,
        api_error_count: 8,
        last_success: None,
        total_latency_ms: 0,
        total_completion_tokens: 0,
        max_completion_tokens: 0,
        max_content_tokens: 0,
        max_thinking_tokens: 0,
    };
    organ.restore_stats(&[("gemma-4b-core".to_string(), stats)]);

    let handle = organ
        .entries
        .get(&BackendId("gemma-4b-core".to_string()))
        .unwrap();
    // Gate should be empty (0 gate-relevant calls), backend stays Healthy
    assert!(
        !handle.is_quality_degraded(),
        "api_errors must not trip the gate"
    );
    let guard = handle.0.lock().unwrap();
    assert!(
        !guard.gate.is_tripped(),
        "gate must not be tripped from api_errors alone"
    );
}

#[test]
fn json_valid_rate_gate_excludes_low_quality_dogs() {
    // K14 gate 2: Dogs with json_valid_rate < 0.5 after baseline (20+ calls) are degraded.
    // Use ApiError (infrastructure) instead of ParseError (model quality) so the
    // ParseFailureGate doesn't trip. This isolates the json_valid_rate gate test.
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("low-quality-dog"));

    // Accumulate 25 calls: 5 successes (20% valid rate), 20 infra failures (ApiError).
    // ApiError doesn't feed ParseFailureGate, so it won't degrade via parse gate.
    for _ in 0..5 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 0,
                completion_tokens: 0,
                thinking_tokens: 0,
            },
        );
    }
    for _ in 0..20 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::ApiError),
        );
    }

    // Verify stats
    let stats = handle.stats_snapshot().unwrap();
    assert_eq!(stats.total_calls, 25);
    assert_eq!(stats.success_count, 5);
    assert_eq!(stats.json_valid_rate(), 0.2);

    // Verify json_valid_rate gate degraded the Dog (not parse gate)
    let guard = handle.0.lock().unwrap();
    assert!(matches!(
        &guard.backend.health,
        BackendHealth::Degraded { reason, .. }
            if reason.contains("json valid rate")
    ));
}

#[test]
fn json_valid_rate_gate_requires_baseline() {
    // K14 gate: json_valid_rate gate only trips after baseline (20 calls).
    // Before baseline, pessimistic K14 defaults are used (0% assumed healthy).
    // Use ApiError so ParseFailureGate doesn't trip independently.
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("new-dog"));

    // 10 calls: 2 successes (20% — below 50%), 8 infra failures (ApiError).
    // ApiError doesn't feed ParseFailureGate, so only json_valid_rate gate matters.
    for _ in 0..2 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 0,
                completion_tokens: 0,
                thinking_tokens: 0,
            },
        );
    }
    for _ in 0..8 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::ApiError),
        );
    }

    // Below baseline (10 < 20 calls): should still be Healthy
    let guard = handle.0.lock().unwrap();
    assert!(matches!(guard.backend.health, BackendHealth::Healthy));
}

#[test]
fn json_valid_rate_gate_recovery() {
    // When json_valid_rate improves to >= 0.5, Dog recovers to Healthy.
    // Use ApiError so only json_valid_rate gate is being tested, not ParseFailureGate.
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("recovering-dog"));

    // 20 calls: 5 successes (25% — below gate), 15 ApiErrors (infrastructure, no parse gate impact)
    for _ in 0..5 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 0,
                completion_tokens: 0,
                thinking_tokens: 0,
            },
        );
    }
    for _ in 0..15 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::ApiError),
        );
    }

    let guard = handle.0.lock().unwrap();
    assert!(matches!(
        &guard.backend.health,
        BackendHealth::Degraded { reason, .. }
            if reason.contains("json valid rate")
    ));
    drop(guard);

    // Add 30 successes (now 35/50 = 70% — above gate)
    for _ in 0..30 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 0,
                completion_tokens: 0,
                thinking_tokens: 0,
            },
        );
    }

    // Should recover to Healthy (json_valid_rate improved from 25% to 70%)
    let guard = handle.0.lock().unwrap();
    assert!(matches!(guard.backend.health, BackendHealth::Healthy));
}

#[test]
fn restore_stats_does_not_replay_gates_at_boot() {
    // B5 amnesia fix: gates don't replay at boot. A Dog with model-quality failures
    // in its persisted stats should have those stats loaded but gate stays fresh.
    // This prevents blocking /judge on first boot after any previous failure.
    let mut organ = InferenceOrgan::boot_empty();
    let _handle = organ.register_backend(make_backend("reviving-dog"));

    let stats = DogStats {
        total_calls: 10,
        success_count: 2,
        zero_flood_count: 3,
        collapse_count: 0,
        parse_error_count: 5,
        timeout_count: 0,
        api_error_count: 0,
        last_success: None,
        total_latency_ms: 0,
        total_completion_tokens: 0,
        max_completion_tokens: 0,
        max_content_tokens: 0,
        max_thinking_tokens: 0,
    };
    organ.restore_stats(&[("reviving-dog".to_string(), stats)]);

    let handle = organ
        .entries
        .get(&BackendId("reviving-dog".to_string()))
        .unwrap();

    // Gate counters are reset at boot (quality gate starts fresh) — only
    // calibration fields (max_content_tokens, max_thinking_tokens, last_success)
    // are restored. success_count=0 is correct: Dogs rebuild reputation from requests.
    let guard = handle.0.lock().unwrap();
    assert_eq!(guard.stats.success_count, 0, "gate counters reset at boot");

    // But gate stays empty — doesn't replay failures
    assert!(
        !guard.gate.is_tripped(),
        "gate must NOT replay at boot — Dogs rebuild reputation from requests"
    );
}

// ── Dynamic budget tests ──────────────────────────────────

#[test]
fn completion_budget_not_available_before_baseline() {
    let stats = DogStats::new();
    assert!(
        stats.completion_budget().is_none(),
        "no budget before 20 calls"
    );
}

#[test]
fn completion_budget_derived_from_observed_max() {
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("budget-dog"));

    // Accumulate 25 calls with varying completion tokens
    for ct in [
        150, 200, 180, 220, 195, 210, 175, 190, 205, 185, 200, 210, 195, 180, 220, 200, 190, 205,
        185, 200, 210, 195, 180, 190, 250,
    ] {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 100,
                completion_tokens: ct,
                thinking_tokens: 0,
            },
        );
    }

    let stats = handle.stats_snapshot().unwrap();
    assert!(stats.is_baseline_established());
    assert_eq!(stats.max_completion_tokens, 250);

    let budget = stats.completion_budget().unwrap();
    // max_content=250, max_thinking=0 → calibrated path: ceil(250*1.2) + 0 = 300
    assert_eq!(budget, 300, "budget = content*1.2 (no thinking overhead)");
}

#[test]
fn tok_per_sec_computed_from_history() {
    let mut stats = DogStats::new();
    // Simulate 10 calls: 200 tokens each, 1000ms each → 2000 tok / 10s = 200 tok/s
    for _ in 0..10 {
        stats.record_success_with_latency(1000);
        stats.record_completion_tokens(200, 0);
    }

    let tps = stats.tok_per_sec().unwrap();
    assert!((tps - 200.0).abs() < 1.0, "expected ~200 tok/s, got {tps}");
}

#[test]
fn scoring_with_budget_overrides_default() {
    use crate::domain::chat::InferenceProfile;
    // Default scoring
    assert_eq!(InferenceProfile::SCORING.max_tokens(), 1024);
    // Dynamic budget
    let profile = InferenceProfile::scoring_with_budget(6000);
    assert_eq!(profile.max_tokens(), 6000);
}

#[test]
fn thinking_budget_accounts_for_thinking_overhead() {
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("gemma-thinking"));

    // Simulate Gemma: 400 combined, 370 thinking + 30 content
    for _ in 0..25 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 5000,
                completion_tokens: 400,
                thinking_tokens: 370,
            },
        );
    }

    let stats = handle.stats_snapshot().unwrap();
    assert!(stats.is_baseline_established());
    assert_eq!(stats.max_thinking_tokens, 370);
    assert_eq!(stats.max_content_tokens, 30);

    let budget = stats.completion_budget().unwrap();
    // content_budget = ceil(30*1.2) = 36, thinking_budget = ceil(370*1.5) = 555
    // total = 591 — derived from data, no magic floor
    assert_eq!(budget, 591, "budget = content*1.2 + thinking*1.5");
    assert!(budget > 370, "budget must exceed thinking overhead");
}

#[test]
fn non_thinking_model_budget_covers_content() {
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("qwen-no-think"));

    // Non-thinking model: 300 content, 0 thinking
    for _ in 0..25 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 200,
                completion_tokens: 300,
                thinking_tokens: 0,
            },
        );
    }

    let stats = handle.stats_snapshot().unwrap();
    assert_eq!(stats.max_content_tokens, 300);
    assert_eq!(stats.max_thinking_tokens, 0);

    let budget = stats.completion_budget().unwrap();
    // max_content=300, max_thinking=0 → calibrated: ceil(300*1.2) = 360
    assert_eq!(budget, 360, "budget = content*1.2, no floor needed");
}

#[test]
fn budget_from_truncated_thinking_model() {
    let mut stats = DogStats::new();
    // Gemma-like: 400 total, 395 thinking, 5 content (truncation scenario)
    for _ in 0..25 {
        stats.record_success_with_latency(1000);
        stats.record_completion_tokens(400, 395);
    }

    let budget = stats.completion_budget().unwrap();
    // content_budget = ceil(5*1.2) = 6, thinking_budget = ceil(395*1.5) = 593
    // total = 599 — enough to cover thinking + content without any magic floor
    assert_eq!(
        budget, 599,
        "budget derived from observed thinking + content"
    );
    assert!(budget > 395, "budget must exceed thinking overhead");
}

// ── Chronic failure gate (H3): promote_if_gate_clear must respect json_valid_rate ──

#[test]
fn promote_if_gate_clear_blocked_by_json_valid_rate() {
    // A Dog degraded by FleetProbe with chronic low json_valid_rate
    // must NOT be promoted when parse gate is clear.
    // This is the Gemma E4B bug: FleetProbe promotes every 30s because
    // parse gate doesn't track timeouts, but json_valid_rate is 16%.
    // Confirmed: 954 Gemma promotions observed in production logs.
    let mut organ = InferenceOrgan::boot_empty();
    let _handle = organ.register_backend(make_backend("slow-dog"));

    let handle = organ
        .entries
        .get(&BackendId("slow-dog".to_string()))
        .unwrap();
    {
        let mut guard = handle.0.lock().unwrap();
        // 25 calls: 5 successes + 20 timeouts → 20% json_valid_rate
        guard.stats = DogStats {
            total_calls: 25,
            success_count: 5,
            zero_flood_count: 0,
            collapse_count: 0,
            parse_error_count: 0,
            timeout_count: 20,
            api_error_count: 0,
            last_success: Some(1_777_212_000_000),
            total_latency_ms: 300000,
            total_completion_tokens: 0,
            max_completion_tokens: 0,
            max_content_tokens: 0,
            max_thinking_tokens: 0,
        };
        // Simulate FleetProbe degradation (different reason string than parse gate)
        guard.backend.health = BackendHealth::Degraded {
            reason: "fleet probe signal".into(),
            since: Instant::now() - Duration::from_secs(31),
        };
    }

    // Parse gate is NOT tripped (no parse failures fed to it)
    assert!(!handle.0.lock().unwrap().gate.is_tripped());

    // promote_if_gate_clear should NOT promote because json_valid_rate is 20% < 50%
    organ.promote_if_gate_clear("slow-dog");

    let guard = handle.0.lock().unwrap();
    assert!(
        matches!(guard.backend.health, BackendHealth::Degraded { .. }),
        "promote must be blocked when json_valid_rate < 0.5"
    );
}

#[test]
fn chronic_timeout_dog_stays_degraded() {
    // Full cycle: Dog starts Healthy, accumulates timeouts, gets degraded
    // by json_valid_rate gate, FleetProbe tries to promote, promotion blocked.
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("chronic-timeout-dog"));

    // 5 successes + 20 timeouts → 20% json_valid_rate
    for _ in 0..5 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Success {
                elapsed_ms: 50000,
                completion_tokens: 200,
                thinking_tokens: 0,
            },
        );
    }
    for _ in 0..20 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::Timeout),
        );
    }

    // json_valid_rate = 5/25 = 20% → K14 gate 2 fires → Degraded
    assert!(
        handle.is_quality_degraded(),
        "Dog must be degraded after 20% success rate with baseline"
    );

    // FleetProbe would call promote_if_gate_clear — must NOT promote
    organ.promote_if_gate_clear("chronic-timeout-dog");
    assert!(
        handle.is_quality_degraded(),
        "FleetProbe must not undo json_valid_rate degradation"
    );
}
