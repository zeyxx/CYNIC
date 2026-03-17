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
