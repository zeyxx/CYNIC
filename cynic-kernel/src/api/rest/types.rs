//! REST API type definitions — request/response structs and shared state.

use serde::{Deserialize, Serialize};
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
    pub rate_limiter: RateLimiter,
    pub judge_limiter: RateLimiter,
}

// ── RATE LIMITER (Token Bucket) ───────────────────────────────

/// Token bucket rate limiter. Tokens regenerate continuously — no burst edge case.
/// Each check() consumes 1 token. Tokens refill at max_per_minute/60 per second.
pub struct RateLimiter {
    state: Mutex<TokenBucket>,
}

struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: std::time::Instant,
}

impl RateLimiter {
    pub fn new(max_per_minute: u64) -> Self {
        let max = max_per_minute as f64;
        Self {
            state: Mutex::new(TokenBucket {
                tokens: max,
                max_tokens: max,
                refill_rate: max / 60.0,
                last_refill: std::time::Instant::now(),
            }),
        }
    }

    /// Returns true if request is allowed (consumes 1 token), false if rate limited.
    pub fn check(&self) -> bool {
        let mut bucket = self.state.lock().unwrap();
        let now = std::time::Instant::now();
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
    use crate::domain::usage::DogUsageTracker;

    #[test]
    fn rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(3);
        assert!(limiter.check());
        assert!(limiter.check());
        assert!(limiter.check());
        assert!(!limiter.check()); // 4th request rejected
    }

    #[test]
    fn usage_tracker_records_tokens() {
        let mut tracker = DogUsageTracker::new();
        tracker.record("dog-a", 100, 50, 200);
        tracker.record("dog-a", 80, 40, 150);
        tracker.record("dog-b", 200, 100, 500);

        assert_eq!(tracker.total_tokens(), 570); // (100+50)+(80+40)+(200+100)
        assert_eq!(tracker.dogs["dog-a"].requests, 2);
        assert_eq!(tracker.dogs["dog-b"].requests, 1);
        assert_eq!(tracker.dogs["dog-a"].total_latency_ms, 350);
    }

    #[test]
    fn usage_tracker_estimated_cost() {
        let mut tracker = DogUsageTracker::new();
        tracker.record("x", 500_000, 500_000, 0); // 1M tokens
        let cost = tracker.estimated_cost_usd();
        assert!((cost - 0.15).abs() < 0.001);
    }

    #[test]
    fn usage_tracker_empty_is_zero() {
        let tracker = DogUsageTracker::new();
        assert_eq!(tracker.total_tokens(), 0);
        assert_eq!(tracker.total_requests, 0);
        assert_eq!(tracker.estimated_cost_usd(), 0.0);
    }
}
