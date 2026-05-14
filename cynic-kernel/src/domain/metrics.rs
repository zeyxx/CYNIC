//! Pipeline metrics — atomic counters for Prometheus-compatible /metrics endpoint.
//! No external crate — pure atomics + text format.

use std::fmt::Write;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global pipeline metrics. Thread-safe via atomics.
#[derive(Debug)]
pub struct Metrics {
    pub verdicts_total: AtomicU64,
    pub cache_hits_total: AtomicU64,
    pub cache_misses_total: AtomicU64,
    pub dog_evaluations_total: AtomicU64,
    pub dog_failures_total: AtomicU64,
    pub crystal_observations_total: AtomicU64,
    pub crystal_gate_quorum_blocked: AtomicU64,
    pub crystal_gate_contested: AtomicU64,
    pub crystal_gate_disputed: AtomicU64,
    pub crystal_observe_failed: AtomicU64,
    pub observation_post_success_total: AtomicU64,
    pub observation_post_failed: AtomicU64,
    pub embedding_successes_total: AtomicU64,
    pub embedding_failures_total: AtomicU64,
    // ── Metabolism: the organism sensing its own data flow ──
    pub observations_ingested_total: AtomicU64,
    pub observations_dropped_total: AtomicU64,
    pub nightshift_digested_total: AtomicU64,
    pub nightshift_errors_total: AtomicU64,
    pub domain_gate_skipped_total: AtomicU64,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            verdicts_total: AtomicU64::new(0),
            cache_hits_total: AtomicU64::new(0),
            cache_misses_total: AtomicU64::new(0),
            dog_evaluations_total: AtomicU64::new(0),
            dog_failures_total: AtomicU64::new(0),
            crystal_observations_total: AtomicU64::new(0),
            crystal_gate_quorum_blocked: AtomicU64::new(0),
            crystal_gate_contested: AtomicU64::new(0),
            crystal_gate_disputed: AtomicU64::new(0),
            crystal_observe_failed: AtomicU64::new(0),
            observation_post_success_total: AtomicU64::new(0),
            observation_post_failed: AtomicU64::new(0),
            embedding_successes_total: AtomicU64::new(0),
            embedding_failures_total: AtomicU64::new(0),
            observations_ingested_total: AtomicU64::new(0),
            observations_dropped_total: AtomicU64::new(0),
            nightshift_digested_total: AtomicU64::new(0),
            nightshift_errors_total: AtomicU64::new(0),
            domain_gate_skipped_total: AtomicU64::new(0),
        }
    }

    pub fn inc_verdict(&self) {
        self.verdicts_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_cache_hit(&self) {
        self.cache_hits_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_cache_miss(&self) {
        self.cache_misses_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_dog_eval(&self) {
        self.dog_evaluations_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_dog_failure(&self) {
        self.dog_failures_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_crystal_obs(&self) {
        self.crystal_observations_total
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_crystal_quorum_blocked(&self) {
        self.crystal_gate_quorum_blocked
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_crystal_contested(&self) {
        self.crystal_gate_contested.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_crystal_disputed(&self) {
        self.crystal_gate_disputed.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_crystal_observe_failed(&self) {
        self.crystal_observe_failed.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_observation_post_success(&self) {
        self.observation_post_success_total
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_observation_post_failed(&self) {
        self.observation_post_failed.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_embed_ok(&self) {
        self.embedding_successes_total
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_embed_fail(&self) {
        self.embedding_failures_total
            .fetch_add(1, Ordering::Relaxed);
    }
    // ── Metabolism ──
    pub fn inc_observation_ingested(&self) {
        self.observations_ingested_total
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_observation_dropped(&self) {
        self.observations_dropped_total
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn add_nightshift_digested(&self, n: u64) {
        self.nightshift_digested_total
            .fetch_add(n, Ordering::Relaxed);
    }
    pub fn add_nightshift_errors(&self, n: u64) {
        self.nightshift_errors_total.fetch_add(n, Ordering::Relaxed);
    }
    pub fn inc_domain_gate_skipped(&self) {
        self.domain_gate_skipped_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Render Prometheus text exposition format (global counters only).
    /// Per-dog and gauge metrics are appended by the /metrics handler.
    pub fn render_prometheus(&self) -> String {
        let mut out = String::with_capacity(2048);
        prom_counter(
            &mut out,
            "cynic_verdicts_total",
            "Total verdicts issued",
            self.verdicts_total.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_cache_hits_total",
            "Verdict cache hits",
            self.cache_hits_total.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_cache_misses_total",
            "Verdict cache misses",
            self.cache_misses_total.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_dog_evaluations_total",
            "Total Dog evaluation attempts (success + failure)",
            self.dog_evaluations_total.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_dog_failures_total",
            "Total Dog evaluation failures",
            self.dog_failures_total.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_crystal_observations_total",
            "Crystal observations recorded",
            self.crystal_observations_total.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_crystal_gate_quorum_blocked",
            "Crystal observations blocked by quorum gate (voter_count < 2)",
            self.crystal_gate_quorum_blocked.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_crystal_gate_contested",
            "Crystal observations quarantined (disagreement >= phi^-2)",
            self.crystal_gate_contested.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_crystal_gate_disputed",
            "Crystal observations with reduced weight (disputed)",
            self.crystal_gate_disputed.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_crystal_observe_failed",
            "Crystal observation storage failures",
            self.crystal_observe_failed.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_observation_post_success_total",
            "Verdict observations posted successfully (K15 forward loop)",
            self.observation_post_success_total.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_observation_post_failed",
            "Verdict observation post failures (K15 forward loop broken)",
            self.observation_post_failed.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_embedding_successes_total",
            "Successful embedding calls",
            self.embedding_successes_total.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_embedding_failures_total",
            "Failed embedding calls",
            self.embedding_failures_total.load(Ordering::Relaxed),
        );

        // ── Metabolism ──
        prom_counter(
            &mut out,
            "cynic_observations_ingested_total",
            "Observations accepted by /observe endpoint",
            self.observations_ingested_total.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_observations_dropped_total",
            "Observations dropped (503 semaphore exhaustion)",
            self.observations_dropped_total.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_nightshift_digested_total",
            "Observations judged by nightshift (Phase 2)",
            self.nightshift_digested_total.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_nightshift_errors_total",
            "Nightshift judgment failures",
            self.nightshift_errors_total.load(Ordering::Relaxed),
        );
        prom_counter(
            &mut out,
            "cynic_domain_gate_skipped_total",
            "Crystal observations skipped by domain gate (general domain)",
            self.domain_gate_skipped_total.load(Ordering::Relaxed),
        );

        // Derived: digestion ratio
        let ingested = self.observations_ingested_total.load(Ordering::Relaxed);
        let digested = self.nightshift_digested_total.load(Ordering::Relaxed);
        let digestion_ratio = if ingested > 0 {
            digested as f64 / ingested as f64
        } else {
            0.0
        };
        prom_gauge(
            &mut out,
            "cynic_digestion_ratio",
            "Fraction of observations digested by nightshift (digested/ingested)",
            digestion_ratio,
        );

        // Derived: crystallization rate
        let crystal_obs = self.crystal_observations_total.load(Ordering::Relaxed);
        let verdicts = self.verdicts_total.load(Ordering::Relaxed);
        let crystal_rate = if verdicts > 0 {
            crystal_obs as f64 / verdicts as f64
        } else {
            0.0
        };
        prom_gauge(
            &mut out,
            "cynic_crystallization_rate",
            "Fraction of verdicts that produced crystal observations",
            crystal_rate,
        );

        // Derived: cache hit rate
        let hits = self.cache_hits_total.load(Ordering::Relaxed);
        let misses = self.cache_misses_total.load(Ordering::Relaxed);
        let total = hits + misses;
        let rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };
        prom_gauge(
            &mut out,
            "cynic_cache_hit_ratio",
            "Verdict cache hit ratio (hits / total)",
            rate,
        );

        out
    }
}

fn prom_counter(out: &mut String, name: &str, help: &str, value: u64) {
    let _ = writeln!(out, "# HELP {name} {help}");
    let _ = writeln!(out, "# TYPE {name} counter");
    let _ = writeln!(out, "{name} {value}");
}

fn prom_gauge(out: &mut String, name: &str, help: &str, value: f64) {
    let _ = writeln!(out, "# HELP {name} {help}");
    let _ = writeln!(out, "# TYPE {name} gauge");
    let _ = writeln!(out, "{name} {value:.6}");
}

/// Append per-dog metrics from usage tracker data.
/// Called by the /metrics handler with data from DogUsageTracker::merged_dogs().
pub fn append_dog_metrics(
    out: &mut String,
    dogs: &[(String, u64, u64, u64, u64)], // (id, requests, failures, total_latency_ms, total_tokens)
    circuit_states: &[(String, String, u32)], // (id, state, consecutive_failures)
) {
    // Per-dog request count
    let _ = writeln!(
        out,
        "# HELP cynic_dog_requests_total Total requests per Dog"
    );
    let _ = writeln!(out, "# TYPE cynic_dog_requests_total counter");
    for (id, requests, _, _, _) in dogs {
        let _ = writeln!(out, "cynic_dog_requests_total{{dog=\"{id}\"}} {requests}");
    }

    // Per-dog failure count
    let _ = writeln!(out, "# HELP cynic_dog_failures Per-dog evaluation failures");
    let _ = writeln!(out, "# TYPE cynic_dog_failures counter");
    for (id, _, failures, _, _) in dogs {
        let _ = writeln!(out, "cynic_dog_failures{{dog=\"{id}\"}} {failures}");
    }

    // Per-dog token consumption
    let _ = writeln!(
        out,
        "# HELP cynic_dog_tokens_total Total tokens consumed per Dog"
    );
    let _ = writeln!(out, "# TYPE cynic_dog_tokens_total counter");
    for (id, _, _, _, tokens) in dogs {
        let _ = writeln!(out, "cynic_dog_tokens_total{{dog=\"{id}\"}} {tokens}");
    }

    // Circuit breaker state: 0=closed, 1=open, 2=half-open
    let _ = writeln!(
        out,
        "# HELP cynic_dog_circuit_breaker Circuit breaker state (0=closed, 1=open, 2=half-open)"
    );
    let _ = writeln!(out, "# TYPE cynic_dog_circuit_breaker gauge");
    for (id, state, _) in circuit_states {
        let val = match state.as_str() {
            "closed" => 0,
            "open" | "critical" => 1,
            "half-open" => 2,
            _ => 0,
        };
        let _ = writeln!(out, "cynic_dog_circuit_breaker{{dog=\"{id}\"}} {val}");
    }
}

/// Append organ quality metrics from DogStats snapshots.
pub fn append_organ_metrics(
    out: &mut String,
    snapshots: &[(String, crate::domain::dog_health::DogStats)],
) {
    use std::fmt::Write;

    // JSON valid rate per Dog (gauge)
    let _ = writeln!(
        out,
        "# HELP cynic_dog_json_valid_rate Fraction of valid JSON responses per Dog"
    );
    let _ = writeln!(out, "# TYPE cynic_dog_json_valid_rate gauge");
    for (id, stats) in snapshots {
        let _ = writeln!(
            out,
            "cynic_dog_json_valid_rate{{dog=\"{id}\"}} {:.6}",
            stats.json_valid_rate()
        );
    }

    // Capability limit rate per Dog (gauge)
    let _ = writeln!(
        out,
        "# HELP cynic_dog_capability_limit_rate Fraction of capability-limit failures per Dog"
    );
    let _ = writeln!(out, "# TYPE cynic_dog_capability_limit_rate gauge");
    for (id, stats) in snapshots {
        let _ = writeln!(
            out,
            "cynic_dog_capability_limit_rate{{dog=\"{id}\"}} {:.6}",
            stats.capability_limit_rate()
        );
    }

    // Total calls per Dog (counter)
    let _ = writeln!(
        out,
        "# HELP cynic_dog_organ_total Total organ-tracked evaluations per Dog"
    );
    let _ = writeln!(out, "# TYPE cynic_dog_organ_total counter");
    for (id, stats) in snapshots {
        let _ = writeln!(
            out,
            "cynic_dog_organ_total{{dog=\"{id}\"}} {}",
            stats.total_calls
        );
    }

    // Quality failures by mode (counter)
    let _ = writeln!(
        out,
        "# HELP cynic_dog_quality_failures Dog quality failures by failure mode"
    );
    let _ = writeln!(out, "# TYPE cynic_dog_quality_failures counter");
    for (id, stats) in snapshots {
        let _ = writeln!(
            out,
            "cynic_dog_quality_failures{{dog=\"{id}\",mode=\"zero_flood\"}} {}",
            stats.zero_flood_count
        );
        let _ = writeln!(
            out,
            "cynic_dog_quality_failures{{dog=\"{id}\",mode=\"collapse\"}} {}",
            stats.collapse_count
        );
        let _ = writeln!(
            out,
            "cynic_dog_quality_failures{{dog=\"{id}\",mode=\"parse_error\"}} {}",
            stats.parse_error_count
        );
        let _ = writeln!(
            out,
            "cynic_dog_quality_failures{{dog=\"{id}\",mode=\"timeout\"}} {}",
            stats.timeout_count
        );
        let _ = writeln!(
            out,
            "cynic_dog_quality_failures{{dog=\"{id}\",mode=\"api_error\"}} {}",
            stats.api_error_count
        );
    }

    // Mean latency per Dog (gauge)
    let _ = writeln!(
        out,
        "# HELP cynic_dog_mean_latency_ms Mean successful evaluation latency in milliseconds"
    );
    let _ = writeln!(out, "# TYPE cynic_dog_mean_latency_ms gauge");
    for (id, stats) in snapshots {
        let _ = writeln!(
            out,
            "cynic_dog_mean_latency_ms{{dog=\"{id}\"}} {:.1}",
            stats.mean_latency_ms()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_metrics_all_zero() {
        let m = Metrics::new();
        assert_eq!(m.verdicts_total.load(Ordering::Relaxed), 0);
        assert_eq!(m.cache_hits_total.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn increment_and_read() {
        let m = Metrics::new();
        m.inc_verdict();
        m.inc_verdict();
        m.inc_cache_hit();
        assert_eq!(m.verdicts_total.load(Ordering::Relaxed), 2);
        assert_eq!(m.cache_hits_total.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn render_prometheus_contains_counters() {
        let m = Metrics::new();
        m.inc_verdict();
        m.inc_cache_hit();
        m.inc_cache_miss();
        m.inc_cache_miss();
        let output = m.render_prometheus();
        assert!(output.contains("cynic_verdicts_total 1"));
        assert!(output.contains("cynic_cache_hits_total 1"));
        assert!(output.contains("cynic_cache_misses_total 2"));
        assert!(output.contains("cynic_cache_hit_ratio"));
        assert!(output.contains("# TYPE cynic_verdicts_total counter"));
    }

    #[test]
    fn cache_hit_ratio_zero_when_empty() {
        let m = Metrics::new();
        let output = m.render_prometheus();
        assert!(output.contains("cynic_cache_hit_ratio 0.000000"));
    }

    #[test]
    fn append_dog_metrics_formats_correctly() {
        let mut out = String::new();
        let dogs = vec![
            ("gemini".to_string(), 10u64, 2u64, 5000u64, 15000u64),
            ("sovereign".to_string(), 5u64, 0u64, 10000u64, 8000u64),
        ];
        let circuits = vec![
            ("gemini".to_string(), "closed".to_string(), 0u32),
            ("sovereign".to_string(), "open".to_string(), 3u32),
        ];
        append_dog_metrics(&mut out, &dogs, &circuits);
        assert!(out.contains("cynic_dog_requests_total{dog=\"gemini\"} 10"));
        assert!(out.contains("cynic_dog_circuit_breaker{dog=\"sovereign\"} 1"));
    }

    #[test]
    fn organ_metrics_renders_correctly() {
        let stats = crate::domain::dog_health::DogStats::new();
        let snapshots = vec![("test-dog".to_string(), stats)];
        let mut out = String::new();
        append_organ_metrics(&mut out, &snapshots);
        assert!(out.contains("cynic_dog_json_valid_rate{dog=\"test-dog\"}"));
        assert!(out.contains("cynic_dog_capability_limit_rate{dog=\"test-dog\"}"));
        assert!(out.contains("cynic_dog_organ_total{dog=\"test-dog\"}"));
        assert!(out.contains("cynic_dog_quality_failures{dog=\"test-dog\",mode=\"zero_flood\"}"));
        assert!(out.contains("cynic_dog_mean_latency_ms{dog=\"test-dog\"}"));
    }
}
