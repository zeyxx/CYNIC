//! DogUsageTracker — tracks token consumption and request counts per Dog.
//! In-memory for hot path + periodic flush to SurrealDB for persistence.

use std::collections::HashMap;

/// Tracks token consumption and request counts per Dog.
/// Cumulative totals survive restarts via load_from_storage / flush_to_storage.
pub struct DogUsageTracker {
    pub dogs: HashMap<String, DogUsage>,
    pub boot_time: chrono::DateTime<chrono::Utc>,
    pub total_requests: u64,
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
            boot_time: chrono::Utc::now(),
            total_requests: 0,
            historical: HashMap::new(),
            historical_requests: 0,
        }
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

    /// Estimated cost in USD (rough average: $0.15/1M tokens)
    pub fn estimated_cost_usd(&self) -> f64 {
        self.total_tokens() as f64 * 0.15 / 1_000_000.0
    }

    pub fn uptime_seconds(&self) -> i64 {
        (chrono::Utc::now() - self.boot_time).num_seconds()
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
    fn record_failure_increments_only_failures() {
        let mut t = DogUsageTracker::new();
        t.record("a", 10, 5, 100);
        t.record_failure("a");
        t.record_failure("a");

        assert_eq!(t.dogs["a"].failures, 2);
        assert_eq!(t.dogs["a"].requests, 1); // failures don't count as requests
        assert_eq!(t.total_requests, 1);
    }

    #[test]
    fn record_failure_creates_entry_if_absent() {
        let mut t = DogUsageTracker::new();
        t.record_failure("new-dog");
        assert_eq!(t.dogs["new-dog"].failures, 1);
        assert_eq!(t.dogs["new-dog"].requests, 0);
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

    // ── estimated_cost_usd ───────────────────────────────

    #[test]
    fn estimated_cost_includes_historical() {
        let mut t = DogUsageTracker::new();
        t.load_historical(&[serde_json::json!({
            "dog_id": "a", "prompt_tokens": 500000, "completion_tokens": 500000,
            "requests": 1, "failures": 0, "total_latency_ms": 0
        })]);
        // 1M tokens × $0.15/1M = $0.15
        assert!((t.estimated_cost_usd() - 0.15).abs() < 0.001);
    }
}
