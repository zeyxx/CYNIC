// Integration tests for organ health module.
// Tests DogStats + ParseFailureGate as exported from the crate.
use cynic_kernel::organ::health::*;

#[test]
fn welford_empty_stddev_placeholder() {
    // Phase 1 uses counters. Welford is Phase 2. This test documents the difference.
    let s = DogStats::new();
    assert_eq!(s.total_calls, 0);
    assert_eq!(s.json_valid_rate(), 0.0); // K14: unknown = 0.0
}

#[test]
fn dog_stats_new_is_in_learning_mode() {
    let stats = DogStats::new();
    assert!(!stats.is_baseline_established());
}

#[test]
fn dog_stats_after_20_calls_baseline_established() {
    let mut stats = DogStats::new();
    for _ in 0..20 {
        stats.record_success();
    }
    assert!(stats.is_baseline_established());
}

#[test]
fn dog_stats_json_valid_rate_tracks_failures() {
    let mut stats = DogStats::new();
    for _ in 0..8 {
        stats.record_success();
    }
    for _ in 0..2 {
        stats.record_failure(ScoreFailureKind::ZeroFlood);
    }
    assert!((stats.json_valid_rate() - 0.8).abs() < 0.001);
}

#[test]
fn dog_stats_distinguishes_failure_kinds() {
    let mut stats = DogStats::new();
    stats.record_failure(ScoreFailureKind::ZeroFlood);
    stats.record_failure(ScoreFailureKind::Collapse);
    stats.record_failure(ScoreFailureKind::ParseError);
    stats.record_failure(ScoreFailureKind::Timeout);
    assert_eq!(stats.zero_flood_count, 1);
    assert_eq!(stats.collapse_count, 1);
    assert_eq!(stats.parse_error_count, 1);
    assert_eq!(stats.timeout_count, 1);
    assert_eq!(stats.total_calls, 4);
    assert_eq!(stats.success_count, 0);
}

#[test]
fn dog_stats_capability_limit_rate() {
    let mut stats = DogStats::new();
    for _ in 0..5 {
        stats.record_success();
    }
    for _ in 0..3 {
        stats.record_failure(ScoreFailureKind::ZeroFlood);
    }
    for _ in 0..2 {
        stats.record_failure(ScoreFailureKind::Collapse);
    }
    // 5/10 capability limits
    assert!((stats.capability_limit_rate() - 0.5).abs() < 0.001);
}

#[test]
fn parse_failure_gate_trips_at_50_percent() {
    let mut gate = ParseFailureGate::new();
    for _ in 0..5 {
        gate.record_success();
    }
    assert!(!gate.is_tripped());
    for _ in 0..6 {
        gate.record_failure();
    }
    // 6 failures out of 11 > 50%
    assert!(gate.is_tripped());
}

#[test]
fn parse_failure_gate_sliding_window() {
    let mut gate = ParseFailureGate::new();
    // Fill window with failures
    for _ in 0..10 {
        gate.record_failure();
    }
    assert!(gate.is_tripped());
    // Overwrite with successes
    for _ in 0..10 {
        gate.record_success();
    }
    assert!(!gate.is_tripped());
}

#[test]
fn parse_failure_gate_needs_5_samples() {
    let mut gate = ParseFailureGate::new();
    for _ in 0..4 {
        gate.record_failure();
    }
    assert!(!gate.is_tripped()); // < 5 samples → never trips
}
