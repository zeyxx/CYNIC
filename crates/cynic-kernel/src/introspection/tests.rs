//! Tests for introspection.

use super::*;
use crate::domain::metrics::Metrics;
use crate::domain::organ::{Metric, MetricKind, MetricValue, OrganSnapshot as OSnap};
use crate::domain::state_log::{
    DogSnapshot, OrganSnapshot, ResourceSnapshot, StateBlock, SystemSnapshot, GENESIS_HASH,
};
use crate::domain::storage::NullStorage;

fn make_rtk_snapshot(savings_pct: f64) -> Vec<(String, OSnap)> {
    vec![(
        "rtk".into(),
        OSnap {
            taken_at: chrono::Utc::now(),
            metrics: vec![Metric {
                key: "savings_pct".into(),
                value: MetricValue::F64(savings_pct),
                kind: MetricKind::Gauge,
                unit: None,
            }],
        },
    )]
}

fn make_tailscale_snapshot(gpu_online: bool) -> Vec<(String, OSnap)> {
    vec![(
        "tailscale".into(),
        OSnap {
            taken_at: chrono::Utc::now(),
            metrics: vec![Metric {
                key: "node_cynic-gpu_online".into(),
                value: MetricValue::Bool(gpu_online),
                kind: MetricKind::Gauge,
                unit: None,
            }],
        },
    )]
}

#[test]
fn metabolism_low_savings_triggers_alert() {
    let alerts = check_sense_health(&make_rtk_snapshot(25.0));
    assert!(alerts.iter().any(|a| a.kind == "metabolism_anomaly"));
}

#[test]
fn metabolism_healthy_savings_no_alert() {
    let alerts = check_sense_health(&make_rtk_snapshot(85.0));
    assert!(alerts.iter().all(|a| a.kind != "metabolism_anomaly"));
}

#[test]
fn fleet_drift_gpu_offline_triggers_alert() {
    let alerts = check_sense_health(&make_tailscale_snapshot(false));
    assert!(alerts.iter().any(|a| a.kind == "fleet_drift"));
}

#[test]
fn fleet_drift_gpu_online_no_alert() {
    let alerts = check_sense_health(&make_tailscale_snapshot(true));
    assert!(alerts.iter().all(|a| a.kind != "fleet_drift"));
}

#[tokio::test]
async fn healthy_system_produces_no_alerts() {
    let metrics = Metrics::new();
    metrics.inc_verdict();
    metrics.inc_dog_eval();
    metrics.inc_dog_eval();
    metrics.inc_embed_ok();
    let alerts = analyze(
        &NullStorage,
        &metrics,
        &None,
        &[],
        &std::collections::HashMap::new(),
    )
    .await;
    assert!(alerts.iter().all(|a| a.kind == "storage_down"));
}

#[tokio::test]
async fn high_failure_rate_triggers_alert() {
    let metrics = Metrics::new();
    for _ in 0..20 {
        metrics.inc_dog_eval();
    }
    for _ in 0..10 {
        metrics.inc_dog_failure();
    }
    let alerts = analyze(
        &NullStorage,
        &metrics,
        &None,
        &[],
        &std::collections::HashMap::new(),
    )
    .await;
    assert!(alerts.iter().any(|a| a.kind == "dog_failure_rate"));
}

#[tokio::test]
async fn no_pipeline_run_in_introspection() {
    let metrics = Metrics::new();
    for _ in 0..20 {
        metrics.inc_dog_eval();
    }
    for _ in 0..15 {
        metrics.inc_dog_failure();
    }
    let alerts = analyze(
        &NullStorage,
        &metrics,
        &None,
        &[],
        &std::collections::HashMap::new(),
    )
    .await;
    assert!(alerts.iter().any(|a| a.kind == "dog_failure_rate"));
}

#[tokio::test]
async fn failure_rate_never_exceeds_100_percent() {
    let metrics = Metrics::new();
    for _ in 0..15 {
        metrics.inc_dog_eval();
    }
    for _ in 0..12 {
        metrics.inc_dog_failure();
    }
    let alerts = analyze(
        &NullStorage,
        &metrics,
        &None,
        &[],
        &std::collections::HashMap::new(),
    )
    .await;
    let dog_alert = alerts.iter().find(|a| a.kind == "dog_failure_rate");
    assert!(dog_alert.is_some(), "should trigger failure rate alert");
    let msg = &dog_alert.unwrap().message;
    let rate_str = msg
        .split('%')
        .next()
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap();
    let rate: f64 = rate_str.parse().unwrap();
    assert!(
        rate <= 100.0,
        "failure rate must never exceed 100%, got {rate}%"
    );
    assert!((rate - 80.0).abs() < 0.1, "expected 80%, got {rate}%");
}

fn make_block(
    seq: u64,
    prev_hash: &str,
    status: &str,
    healthy: usize,
    total: usize,
    dogs: Vec<DogSnapshot>,
    organs: Vec<OrganSnapshot>,
) -> StateBlock {
    StateBlock::new(
        seq,
        prev_hash.to_string(),
        dogs,
        SystemSnapshot {
            status: status.into(),
            healthy_dogs: healthy,
            total_dogs: total,
            verdict_count: 100,
            total_tokens: 50000,
            crystals_forming: 5,
            crystals_crystallized: 2,
        },
        ResourceSnapshot {
            cpu_pct: 10.0,
            memory_used_gb: 14.0,
            disk_avail_gb: 70.0,
            uptime_secs: 3600,
        },
        organs,
        vec![],
    )
}

fn make_dog(id: &str, success_rate: f64) -> DogSnapshot {
    DogSnapshot {
        id: id.into(),
        circuit: "closed".into(),
        success_rate,
        mean_latency_ms: 1500.0,
        failures: 10,
        last_failure_reason: None,
    }
}

fn make_organ(source: &str, silence_secs: u64) -> OrganSnapshot {
    let last_obs = chrono::Utc::now() - chrono::Duration::seconds(silence_secs as i64);
    OrganSnapshot {
        source: source.into(),
        last_observation: last_obs.to_rfc3339(),
        total_observations: 100,
        silence_secs,
    }
}

#[test]
fn m1_ephemeral_session_no_silence_alert() {
    let prev = make_block(0, GENESIS_HASH, "sovereign", 5, 5, vec![], vec![]);
    let curr = make_block(
        1,
        &prev.hash,
        "sovereign",
        5,
        5,
        vec![],
        vec![
            make_organ("claude-abc123-def", 999999),
            make_organ("gemini-xyz-456", 999999),
        ],
    );
    let alerts = analyze_state_log_trends(&prev, &curr);
    assert!(
        alerts.iter().all(|a| a.kind != "organ_silence"),
        "ephemeral sessions must not trigger organ_silence, got: {alerts:?}"
    );
}

#[test]
fn m1_permanent_organ_silence_alert() {
    let prev = make_block(0, GENESIS_HASH, "sovereign", 5, 5, vec![], vec![]);
    let curr = make_block(
        1,
        &prev.hash,
        "sovereign",
        5,
        5,
        vec![],
        vec![
            make_organ("hermes-gateway", 7200),
            make_organ("organ-cultscreener", 86401),
            make_organ("kernel", 600),
        ],
    );
    let alerts = analyze_state_log_trends(&prev, &curr);
    let silence_alerts: Vec<_> = alerts
        .iter()
        .filter(|a| a.kind == "organ_silence")
        .collect();
    assert_eq!(
        silence_alerts.len(),
        2,
        "hermes-gateway + organ-cultscreener should alert"
    );
    assert!(
        silence_alerts
            .iter()
            .any(|a| a.message.contains("hermes-gateway") && a.severity == "warning"),
        "hermes-gateway should be warning"
    );
    assert!(
        silence_alerts
            .iter()
            .any(|a| a.message.contains("organ-cultscreener") && a.severity == "critical"),
        "organ-cultscreener >24h should be critical"
    );
}

#[test]
fn m1_retired_sources_no_silence_alert() {
    let prev = make_block(0, GENESIS_HASH, "sovereign", 5, 5, vec![], vec![]);
    let curr = make_block(
        1,
        &prev.hash,
        "sovereign",
        5,
        5,
        vec![],
        vec![
            make_organ("hermes-hermes", 999999),
            make_organ("hermes-x-proxy", 999999),
            make_organ("hermes-x-organ", 999999),
            make_organ("x-proxy", 999999),
            make_organ("hermes-x", 999999),
        ],
    );
    let alerts = analyze_state_log_trends(&prev, &curr);
    assert!(
        alerts.iter().all(|a| a.kind != "organ_silence"),
        "retired sources must not trigger organ_silence, got: {alerts:?}"
    );
}

#[test]
fn m1_permanent_source_classification() {
    assert!(is_permanent_source("hermes-x"));
    assert!(is_permanent_source("hermes-gateway"));
    assert!(is_permanent_source("x-proxy"));
    assert!(is_permanent_source("kernel"));
    assert!(is_permanent_source("nightshift"));
    assert!(is_permanent_source("watchlist"));
    assert!(is_permanent_source("organ-cultscreener"));

    assert!(!is_permanent_source("claude-abc123-def"));
    assert!(!is_permanent_source("gemini-xyz"));
    assert!(!is_permanent_source("codex-20260415"));
    assert!(!is_permanent_source("unknown"));
    assert!(!is_permanent_source(""));
}

#[test]
fn e1_source_tier_classification() {
    assert_eq!(classify_source_tier("kernel"), "permanent");
    assert_eq!(classify_source_tier("hermes-x"), "permanent");
    assert_eq!(classify_source_tier("x-proxy"), "permanent");
    assert_eq!(classify_source_tier("nightshift"), "permanent");
    assert_eq!(classify_source_tier("watchlist"), "permanent");
    assert_eq!(classify_source_tier("organ-cultscreener"), "permanent");

    assert_eq!(classify_source_tier("cron-telemetry-digest"), "cron");
    assert_eq!(classify_source_tier("cron-anything"), "cron");

    assert_eq!(classify_source_tier("claude-abc123-def"), "session");
    assert_eq!(classify_source_tier("gemini-xyz"), "session");
    assert_eq!(classify_source_tier("kernel-probe"), "session");
    assert_eq!(classify_source_tier("k15-consumer"), "session");
    assert_eq!(classify_source_tier("unknown"), "session");
    assert_eq!(classify_source_tier(""), "session");
}

#[test]
fn dog_degradation_triggers_alert() {
    let dogs_before = vec![make_dog("qwen-7b", 0.95)];
    let dogs_after = vec![make_dog("qwen-7b", 0.60)];
    let prev = make_block(0, GENESIS_HASH, "sovereign", 5, 5, dogs_before, vec![]);
    let curr = make_block(1, &prev.hash, "sovereign", 5, 5, dogs_after, vec![]);
    let alerts = analyze_state_log_trends(&prev, &curr);
    assert!(alerts.iter().any(|a| a.kind == "dog_degradation_trend"));
}

#[test]
fn dog_stable_no_degradation_alert() {
    let dogs_before = vec![make_dog("qwen-7b", 0.95)];
    let dogs_after = vec![make_dog("qwen-7b", 0.90)];
    let prev = make_block(0, GENESIS_HASH, "sovereign", 5, 5, dogs_before, vec![]);
    let curr = make_block(1, &prev.hash, "sovereign", 5, 5, dogs_after, vec![]);
    let alerts = analyze_state_log_trends(&prev, &curr);
    assert!(alerts.iter().all(|a| a.kind != "dog_degradation_trend"));
}

#[test]
fn system_regression_triggers_alert() {
    let prev = make_block(0, GENESIS_HASH, "sovereign", 5, 5, vec![], vec![]);
    let curr = make_block(1, &prev.hash, "degraded", 3, 5, vec![], vec![]);
    let alerts = analyze_state_log_trends(&prev, &curr);
    assert!(alerts.iter().any(|a| a.kind == "system_regression"));
}

#[test]
fn system_stable_no_regression_alert() {
    let prev = make_block(0, GENESIS_HASH, "degraded", 3, 5, vec![], vec![]);
    let curr = make_block(1, &prev.hash, "degraded", 3, 5, vec![], vec![]);
    let alerts = analyze_state_log_trends(&prev, &curr);
    assert!(alerts.iter().all(|a| a.kind != "system_regression"));
}
