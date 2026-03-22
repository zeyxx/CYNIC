//! DogUsageTracker — tracks token consumption and request counts per Dog.
//! In-memory for hot path + periodic flush to SurrealDB for persistence.

use std::collections::HashMap;

/// Tracks token consumption and request counts per Dog.
/// Cumulative totals survive restarts via load_from_storage / flush_to_storage.
pub struct DogUsageTracker {
    pub dogs: HashMap<String, DogUsage>,
    pub boot_time: std::time::Instant,
    pub total_requests: u64,
    /// Per-Dog cost rates (USD per 1M tokens). 0.0 = free.
    cost_rates: HashMap<String, (f64, f64)>, // (input_per_mtok, output_per_mtok)
    /// Accumulated totals loaded from DB at boot (pre-boot history).
    historical: HashMap<String, DogUsage>,
    historical_requests: u64,
}

#[derive(Default, Clone)]
pub struct DogUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub requests: u64,
    pub failures: u64,
    pub total_latency_ms: u64,
}

impl DogUsage {
    pub fn total_tokens(&self) -> u64 {
        self.prompt_tokens + self.completion_tokens
    }
}

impl Default for DogUsageTracker {
    fn default() -> Self { Self::new() }
}

impl DogUsageTracker {
    pub fn new() -> Self {
        Self {
            dogs: HashMap::new(),
            boot_time: std::time::Instant::now(),
            total_requests: 0,
            cost_rates: HashMap::new(),
            historical: HashMap::new(),
            historical_requests: 0,
        }
    }

    /// Register cost rates for a Dog. Called at boot from backends.toml config.
    pub fn set_cost_rate(&mut self, dog_id: &str, input_per_mtok: f64, output_per_mtok: f64) {
        self.cost_rates.insert(dog_id.to_string(), (input_per_mtok, output_per_mtok));
    }

    pub fn record(&mut self, dog_id: &str, prompt: u32, completion: u32, latency_ms: u64) {
        let entry = self.dogs.entry(dog_id.to_string()).or_default();
        entry.prompt_tokens += prompt as u64;
        entry.completion_tokens += completion as u64;
        entry.requests += 1;
        entry.total_latency_ms += latency_ms;
        self.total_requests += 1;
    }

    pub fn record_failure(&mut self, dog_id: &str) {
        let entry = self.dogs.entry(dog_id.to_string()).or_default();
        entry.failures += 1;
        entry.requests += 1;
        self.total_requests += 1;
    }

    /// Total tokens this session only.
    pub fn session_tokens(&self) -> u64 {
        self.dogs.values().map(|d| d.total_tokens()).sum()
    }

    /// All-time total tokens (historical + session).
    pub fn total_tokens(&self) -> u64 {
        let hist: u64 = self.historical.values().map(|d| d.total_tokens()).sum();
        hist + self.session_tokens()
    }

    /// All-time total requests.
    pub fn all_time_requests(&self) -> u64 {
        self.historical_requests + self.total_requests
    }

    /// Estimated cost in USD — per-Dog rates from backends.toml. $0 for sovereign/free-tier.
    pub fn estimated_cost_usd(&self) -> f64 {
        let mut cost = 0.0;
        for (id, usage) in self.merged_dogs().iter() {
            let (input_rate, output_rate) = self.cost_rates.get(id).copied().unwrap_or((0.0, 0.0));
            cost += usage.prompt_tokens as f64 * input_rate / 1_000_000.0;
            cost += usage.completion_tokens as f64 * output_rate / 1_000_000.0;
        }
        cost
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.boot_time.elapsed().as_secs()
    }

    /// Merge per-Dog totals (historical + session) for display.
    pub fn merged_dogs(&self) -> HashMap<String, DogUsage> {
        let mut merged = self.historical.clone();
        for (id, session) in &self.dogs {
            let entry = merged.entry(id.clone()).or_default();
            entry.prompt_tokens += session.prompt_tokens;
            entry.completion_tokens += session.completion_tokens;
            entry.requests += session.requests;
            entry.failures += session.failures;
            entry.total_latency_ms += session.total_latency_ms;
        }
        merged
    }

    /// Load historical totals from DB rows. Called once at boot.
    pub fn load_historical(&mut self, rows: &[serde_json::Value]) {
        for row in rows {
            let dog_id = row["dog_id"].as_str().unwrap_or("").to_string();
            if dog_id.is_empty() { continue; }
            let usage = DogUsage {
                prompt_tokens: row["prompt_tokens"].as_u64().unwrap_or(0),
                completion_tokens: row["completion_tokens"].as_u64().unwrap_or(0),
                requests: row["requests"].as_u64().unwrap_or(0),
                failures: row["failures"].as_u64().unwrap_or(0),
                total_latency_ms: row["total_latency_ms"].as_u64().unwrap_or(0),
            };
            self.historical_requests += usage.requests;
            self.historical.insert(dog_id, usage);
        }
    }

    /// Snapshot current session data as rows for persistence.
    pub fn snapshot(&self) -> Vec<(String, DogUsage)> {
        self.dogs.iter().map(|(id, u)| (id.clone(), u.clone())).collect()
    }

    /// Absolute snapshot for idempotent flush — merges historical + session.
    /// The returned values are the TOTAL per Dog (not deltas).
    /// flush_usage uses SET (not +=) with these, so partial retry is safe.
    pub fn flush_snapshot(&self) -> Vec<(String, DogUsage)> {
        self.merged_dogs().into_iter().collect()
    }

    /// After a successful flush to DB, transfer session counters into historical
    /// and reset session. Without this, total_tokens() under-reports after flush.
    pub fn absorb_flush(&mut self) {
        for (id, session) in self.dogs.drain() {
            let entry = self.historical.entry(id).or_default();
            entry.prompt_tokens += session.prompt_tokens;
            entry.completion_tokens += session.completion_tokens;
            entry.requests += session.requests;
            entry.failures += session.failures;
            entry.total_latency_ms += session.total_latency_ms;
        }
        self.historical_requests += self.total_requests;
        self.total_requests = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── record / record_failure ──────────────────────────

    #[test]
    fn record_accumulates_per_dog() {
        let mut t = DogUsageTracker::new();
        t.record("a", 100, 50, 200);
        t.record("a", 80, 40, 150);
        t.record("b", 200, 100, 500);

        assert_eq!(t.dogs["a"].prompt_tokens, 180);
        assert_eq!(t.dogs["a"].completion_tokens, 90);
        assert_eq!(t.dogs["a"].requests, 2);
        assert_eq!(t.dogs["a"].total_latency_ms, 350);
        assert_eq!(t.dogs["b"].requests, 1);
        assert_eq!(t.total_requests, 3);
    }

    #[test]
    fn record_failure_increments_failures_and_requests() {
        let mut t = DogUsageTracker::new();
        t.record("a", 10, 5, 100);
        t.record_failure("a");
        t.record_failure("a");

        assert_eq!(t.dogs["a"].failures, 2);
        assert_eq!(t.dogs["a"].requests, 3); // 1 success + 2 failures = 3 invocations
        assert_eq!(t.total_requests, 3);
    }

    #[test]
    fn record_failure_creates_entry_if_absent() {
        let mut t = DogUsageTracker::new();
        t.record_failure("new-dog");
        assert_eq!(t.dogs["new-dog"].failures, 1);
        assert_eq!(t.dogs["new-dog"].requests, 1); // a failed invocation is still a request
    }

    // ── session_tokens / total_tokens ────────────────────

    #[test]
    fn session_tokens_counts_session_only() {
        let mut t = DogUsageTracker::new();
        // Load historical
        t.load_historical(&[serde_json::json!({
            "dog_id": "a", "prompt_tokens": 1000, "completion_tokens": 500,
            "requests": 10, "failures": 0, "total_latency_ms": 5000
        })]);
        // Record session
        t.record("a", 100, 50, 200);

        assert_eq!(t.session_tokens(), 150); // only session
        assert_eq!(t.total_tokens(), 1650);  // historical + session
    }

    // ── all_time_requests ────────────────────────────────

    #[test]
    fn all_time_requests_combines_historical_and_session() {
        let mut t = DogUsageTracker::new();
        t.load_historical(&[serde_json::json!({
            "dog_id": "a", "prompt_tokens": 0, "completion_tokens": 0,
            "requests": 42, "failures": 0, "total_latency_ms": 0
        })]);
        t.record("a", 10, 5, 100);
        t.record("b", 10, 5, 100);

        assert_eq!(t.all_time_requests(), 44); // 42 historical + 2 session
    }

    // ── load_historical ──────────────────────────────────

    #[test]
    fn load_historical_from_json_rows() {
        let mut t = DogUsageTracker::new();
        let rows = vec![
            serde_json::json!({
                "dog_id": "gemini", "prompt_tokens": 5000, "completion_tokens": 2000,
                "requests": 50, "failures": 3, "total_latency_ms": 25000
            }),
            serde_json::json!({
                "dog_id": "sovereign", "prompt_tokens": 3000, "completion_tokens": 1000,
                "requests": 30, "failures": 1, "total_latency_ms": 15000
            }),
        ];
        t.load_historical(&rows);

        assert_eq!(t.total_tokens(), 11000); // (5000+2000)+(3000+1000)
        assert_eq!(t.all_time_requests(), 80); // 50+30
        assert_eq!(t.session_tokens(), 0); // nothing in session yet
    }

    #[test]
    fn load_historical_skips_empty_dog_id() {
        let mut t = DogUsageTracker::new();
        t.load_historical(&[serde_json::json!({
            "dog_id": "", "prompt_tokens": 999, "completion_tokens": 0,
            "requests": 1, "failures": 0, "total_latency_ms": 0
        })]);
        assert_eq!(t.total_tokens(), 0);
        assert_eq!(t.all_time_requests(), 0);
    }

    #[test]
    fn load_historical_handles_missing_fields() {
        let mut t = DogUsageTracker::new();
        t.load_historical(&[serde_json::json!({
            "dog_id": "partial"
            // all other fields missing → default to 0
        })]);
        assert_eq!(t.total_tokens(), 0);
        assert!(t.all_time_requests() == 0);
    }

    // ── absorb_flush (critical — fixed in 5168ebd) ──────

    #[test]
    fn absorb_flush_transfers_session_to_historical() {
        let mut t = DogUsageTracker::new();
        t.record("a", 100, 50, 200);
        t.record("b", 200, 100, 500);

        let total_before = t.total_tokens();
        t.absorb_flush();

        // Session is drained
        assert!(t.dogs.is_empty());
        assert_eq!(t.total_requests, 0);
        assert_eq!(t.session_tokens(), 0);

        // But total_tokens still reports correctly (now from historical)
        assert_eq!(t.total_tokens(), total_before);
        assert_eq!(t.all_time_requests(), 2);
    }

    #[test]
    fn absorb_flush_accumulates_into_existing_historical() {
        let mut t = DogUsageTracker::new();
        // Boot with historical data
        t.load_historical(&[serde_json::json!({
            "dog_id": "a", "prompt_tokens": 1000, "completion_tokens": 500,
            "requests": 10, "failures": 0, "total_latency_ms": 5000
        })]);
        // Session activity
        t.record("a", 100, 50, 200);

        t.absorb_flush();

        // Historical should now be 1000+100=1100 prompt, 500+50=550 completion
        assert_eq!(t.total_tokens(), 1650); // (1100+550) + 0 session
        assert_eq!(t.all_time_requests(), 11); // 10 + 1
    }

    #[test]
    fn absorb_flush_then_new_session_doesnt_double_count() {
        let mut t = DogUsageTracker::new();
        t.record("a", 100, 50, 200);
        t.absorb_flush();

        // New session activity after flush
        t.record("a", 80, 40, 150);

        // Should be 150 (historical) + 120 (new session) = 270
        assert_eq!(t.total_tokens(), 270);
        assert_eq!(t.all_time_requests(), 2); // 1 flushed + 1 new
    }

    // ── merged_dogs ──────────────────────────────────────

    #[test]
    fn merged_dogs_combines_historical_and_session() {
        let mut t = DogUsageTracker::new();
        t.load_historical(&[serde_json::json!({
            "dog_id": "a", "prompt_tokens": 1000, "completion_tokens": 500,
            "requests": 10, "failures": 1, "total_latency_ms": 5000
        })]);
        t.record("a", 100, 50, 200);
        t.record("b", 200, 100, 500); // new dog, no history

        let merged = t.merged_dogs();
        assert_eq!(merged["a"].prompt_tokens, 1100);
        assert_eq!(merged["a"].completion_tokens, 550);
        assert_eq!(merged["a"].requests, 11);
        assert_eq!(merged["a"].failures, 1);
        assert_eq!(merged["b"].prompt_tokens, 200);
        assert_eq!(merged["b"].requests, 1);
    }

    // ── snapshot ─────────────────────────────────────────

    #[test]
    fn snapshot_returns_session_data() {
        let mut t = DogUsageTracker::new();
        t.record("a", 100, 50, 200);
        t.record("b", 200, 100, 500);

        let snap = t.snapshot();
        assert_eq!(snap.len(), 2);
        let total: u64 = snap.iter().map(|(_, u)| u.total_tokens()).sum();
        assert_eq!(total, 450);
    }

    #[test]
    fn snapshot_is_empty_after_absorb() {
        let mut t = DogUsageTracker::new();
        t.record("a", 100, 50, 200);
        t.absorb_flush();

        let snap = t.snapshot();
        assert!(snap.is_empty());
    }

    // ── flush_snapshot (idempotent) ──────────────────────

    #[test]
    fn flush_snapshot_returns_absolute_totals() {
        let mut t = DogUsageTracker::new();
        t.load_historical(&[serde_json::json!({
            "dog_id": "a", "prompt_tokens": 1000, "completion_tokens": 500,
            "requests": 10, "failures": 0, "total_latency_ms": 5000
        })]);
        t.record("a", 100, 50, 200);
        t.record("b", 200, 100, 500);

        let snap: std::collections::HashMap<String, DogUsage> =
            t.flush_snapshot().into_iter().collect();
        // "a" = historical (1000+500) + session (100+50)
        assert_eq!(snap["a"].prompt_tokens, 1100);
        assert_eq!(snap["a"].completion_tokens, 550);
        // "b" = session only (no history)
        assert_eq!(snap["b"].prompt_tokens, 200);
    }

    #[test]
    fn flush_snapshot_is_idempotent_on_retry() {
        let mut t = DogUsageTracker::new();
        t.record("a", 100, 50, 200);

        let snap1 = t.flush_snapshot();
        // Simulate failed flush: don't call absorb_flush()
        let snap2 = t.flush_snapshot();

        // Same absolute values — SET is idempotent, no double-counting
        let s1: std::collections::HashMap<String, DogUsage> = snap1.into_iter().collect();
        let s2: std::collections::HashMap<String, DogUsage> = snap2.into_iter().collect();
        assert_eq!(s1["a"].prompt_tokens, s2["a"].prompt_tokens);
    }

    // ── estimated_cost_usd ───────────────────────────────

    #[test]
    fn estimated_cost_includes_historical() {
        let mut t = DogUsageTracker::new();
        // Gemini-like pricing: $0.50/1M input, $3.00/1M output
        t.set_cost_rate("a", 0.50, 3.00);
        t.load_historical(&[serde_json::json!({
            "dog_id": "a", "prompt_tokens": 500000, "completion_tokens": 500000,
            "requests": 1, "failures": 0, "total_latency_ms": 0
        })]);
        // 500K input × $0.50/1M + 500K output × $3.00/1M = $0.25 + $1.50 = $1.75
        assert!((t.estimated_cost_usd() - 1.75).abs() < 0.001);
    }

    #[test]
    fn zero_cost_for_sovereign_dogs() {
        let mut t = DogUsageTracker::new();
        // Sovereign/free-tier dogs: no cost rates set (default 0.0)
        t.record("sovereign-ubuntu", 100000, 50000, 500);
        assert!((t.estimated_cost_usd()).abs() < 0.0001);
    }
}
