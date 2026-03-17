//! REST API type definitions — request/response structs and shared state.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

use crate::domain::coord::CoordPort;
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::judge::Judge;

// ── SHARED STATE ───────────────────────────────────────────

pub struct AppState {
    pub judge: Arc<Judge>,
    pub storage: Arc<dyn StoragePort>,
    pub coord: Arc<dyn CoordPort>,
    pub usage: Arc<Mutex<DogUsageTracker>>,
    pub api_key: Option<String>,
    pub rate_limiter: PerIpRateLimiter,
    pub judge_limiter: PerIpRateLimiter,
}

// ── PER-IP RATE LIMITER ──────────────────────────────────────

/// Per-IP token bucket rate limiter. Each IP gets its own bucket.
/// Stale entries are evicted after 2 minutes of inactivity.
pub struct PerIpRateLimiter {
    buckets: Mutex<HashMap<IpAddr, TokenBucket>>,
    max_per_minute: u64,
}

struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: std::time::Instant,
}

impl PerIpRateLimiter {
    pub fn new(max_per_minute: u64) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            max_per_minute,
        }
    }

    /// Returns true if request from this IP is allowed.
    pub fn check(&self, ip: IpAddr) -> bool {
        let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        let now = std::time::Instant::now();

        // Evict stale entries (>2min inactive) to prevent memory leak
        buckets.retain(|_, b| now.duration_since(b.last_refill).as_secs() < 120);

        let max = self.max_per_minute as f64;
        let bucket = buckets.entry(ip).or_insert_with(|| TokenBucket {
            tokens: max,
            max_tokens: max,
            refill_rate: max / 60.0,
            last_refill: now,
        });

        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * bucket.refill_rate).min(bucket.max_tokens);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

// ── REQUEST / RESPONSE TYPES ───────────────────────────────

#[derive(Deserialize)]
pub struct JudgeRequest {
    pub content: String,
    pub context: Option<String>,
    pub domain: Option<String>,
    /// Optional: evaluate with only these Dogs (by ID). If omitted, all Dogs are used.
    pub dogs: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct JudgeResponse {
    pub verdict_id: String,
    pub verdict: String,
    pub q_score: QScoreResponse,
    pub reasoning: ReasoningResponse,
    pub dogs_used: String,
    pub phi_max: f64,
    pub dog_scores: Vec<DogScoreResponse>,
    pub anomaly_detected: bool,
    pub max_disagreement: f64,
    pub anomaly_axiom: Option<String>,
    pub temporal: Option<TemporalResponse>,
    /// BLAKE3 integrity hash of this verdict
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity_hash: Option<String>,
    /// BLAKE3 hash of the previous verdict (chain link)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_hash: Option<String>,
}

#[derive(Serialize)]
pub struct TemporalResponse {
    pub temporal_total: f64,
    pub outlier_perspective: Option<String>,
    pub max_divergence: f64,
    pub perspectives: Vec<TemporalPerspectiveScore>,
}

#[derive(Serialize)]
pub struct TemporalPerspectiveScore {
    pub perspective: String,
    pub q_total: f64,
    pub dog_id: String,
}

#[derive(Serialize)]
pub struct DogScoreResponse {
    pub dog_id: String,
    pub latency_ms: u64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
    pub reasoning: ReasoningResponse,
}

#[derive(Serialize)]
pub struct QScoreResponse {
    pub total: f64,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
}

#[derive(Serialize)]
pub struct ReasoningResponse {
    pub fidelity: String,
    pub phi: String,
    pub verify: String,
    pub culture: String,
    pub burn: String,
    pub sovereignty: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Serialize)]
pub struct DogHealthResponse {
    pub id: String,
    pub kind: String,
    pub circuit: String,
    pub failures: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_allows_within_limit() {
        let limiter = PerIpRateLimiter::new(3);
        let ip: std::net::IpAddr = "127.0.0.1".parse().unwrap();
        assert!(limiter.check(ip));
        assert!(limiter.check(ip));
        assert!(limiter.check(ip));
        assert!(!limiter.check(ip)); // 4th request rejected
        // Different IP gets its own bucket
        let ip2: std::net::IpAddr = "10.0.0.1".parse().unwrap();
        assert!(ip2 != ip);
        assert!(limiter.check(ip2)); // separate bucket, allowed
    }
}
