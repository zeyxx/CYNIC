//! REST API type definitions — request/response structs and shared state.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::domain::coord::CoordPort;
use crate::domain::embedding::EmbeddingPort;
use crate::domain::events::KernelEvent;
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::domain::verdict_cache::VerdictCache;
use crate::domain::metrics::Metrics;
use crate::infra::task_health::TaskHealth;
use crate::introspection::Alert;
use crate::judge::Judge;

// ── SHARED STATE ───────────────────────────────────────────

pub struct AppState {
    pub judge: Arc<Judge>,
    pub storage: Arc<dyn StoragePort>,
    pub coord: Arc<dyn CoordPort>,
    pub embedding: Arc<dyn EmbeddingPort>,
    pub usage: Arc<Mutex<DogUsageTracker>>,
    pub verdict_cache: Arc<VerdictCache>,
    pub task_health: Arc<TaskHealth>,
    pub metrics: Arc<Metrics>,
    pub api_key: Option<String>,
    pub storage_info: StorageInfo,
    pub rate_limiter: PerIpRateLimiter,
    pub judge_limiter: PerIpRateLimiter,
    /// Bounds fire-and-forget background spawns (observe, audit).
    /// Prevents unbounded task accumulation under DB degradation.
    pub bg_semaphore: Arc<tokio::sync::Semaphore>,
    /// Tracks fire-and-forget spawns for drain at shutdown.
    /// Semaphore bounds concurrency; TaskTracker enables wait-for-completion.
    pub bg_tasks: tokio_util::task::TaskTracker,
    /// Latest introspection alerts (updated every 5min by background task).
    /// Empty = healthy system. RwLock: read-heavy (every /health), write every 5min.
    pub introspection_alerts: Arc<std::sync::RwLock<Vec<Alert>>>,
    /// Kernel event bus — broadcast to all SSE/WebSocket subscribers.
    /// Capacity 256: events are small, subscribers should keep up.
    /// Lagging subscribers get BroadcastStreamRecvError::Lagged → skip.
    pub event_tx: tokio::sync::broadcast::Sender<KernelEvent>,
}

/// Storage topology — exposed on authenticated /health for discoverability.
#[derive(Clone)]
pub struct StorageInfo {
    pub namespace: String,
    pub database: String,
}

impl AppState {
    pub fn storage_metrics(&self) -> Option<crate::domain::storage::StorageMetrics> {
        self.storage.metrics()
    }
}

// ── PER-IP RATE LIMITER ──────────────────────────────────────

/// Per-IP token bucket rate limiter. Each IP gets its own bucket.
/// Uses tokio::sync::Mutex — never blocks a tokio worker thread.
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

    /// Evict stale entries — called by background timer, NOT on the hot path.
    /// Previous design ran retain() on every request = O(N) under DDoS.
    pub async fn evict_stale(&self) {
        let mut buckets = self.buckets.lock().await;
        let now = std::time::Instant::now();
        buckets.retain(|_, b| now.duration_since(b.last_refill).as_secs() < 120);
    }

    /// Returns true if request from this IP is allowed.
    pub async fn check(&self, ip: IpAddr) -> bool {
        let mut buckets = self.buckets.lock().await;
        let now = std::time::Instant::now();

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
    /// True if this verdict came from semantic cache (0 API calls consumed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_hit: Option<f64>,
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

    #[tokio::test]
    async fn rate_limiter_allows_within_limit() {
        let limiter = PerIpRateLimiter::new(3);
        let ip: std::net::IpAddr = "127.0.0.1".parse().unwrap();
        assert!(limiter.check(ip).await);
        assert!(limiter.check(ip).await);
        assert!(limiter.check(ip).await);
        assert!(!limiter.check(ip).await); // 4th request rejected
        // Different IP gets its own bucket
        let ip2: std::net::IpAddr = "10.0.0.1".parse().unwrap();
        assert!(ip2 != ip);
        assert!(limiter.check(ip2).await); // separate bucket, allowed
    }
}
