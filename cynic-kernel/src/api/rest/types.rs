//! REST API type definitions — request/response structs and shared state.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::Mutex;

use arc_swap::ArcSwap;

use super::judge_job::JudgeJobStore;
use crate::domain::coord::CoordPort;
use crate::domain::embedding::EmbeddingPort;
use crate::domain::events::KernelEvent;
use crate::domain::metrics::Metrics;
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::domain::verdict_cache::VerdictCache;
use crate::infra::task_health::TaskHealth;
use crate::introspection::Alert;
use crate::judge::Judge;

// ── SHARED STATE ───────────────────────────────────────────

pub struct AppState {
    pub judge: ArcSwap<Judge>,
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
    /// F22: Cached /ready result — avoids DB ping on every probe call.
    /// Updated by readiness_handler when stale (>30s).
    pub ready_cache: ReadyCache,
    /// Bounds fire-and-forget background spawns (observe, audit).
    /// Prevents unbounded task accumulation under DB degradation.
    pub bg_semaphore: Arc<tokio::sync::Semaphore>,
    /// Tracks fire-and-forget spawns for drain at shutdown.
    /// Semaphore bounds concurrency; TaskTracker enables wait-for-completion.
    pub bg_tasks: tokio_util::task::TaskTracker,
    /// Latest introspection alerts (updated every 5min by background task).
    /// Empty = healthy system. RwLock: read-heavy (every /health), write every 5min.
    pub introspection_alerts: Arc<std::sync::RwLock<Vec<Alert>>>,
    /// F23: Bounds concurrent SSE connections to prevent FD exhaustion.
    /// 32 connections is generous for operational monitoring.
    pub sse_semaphore: Arc<tokio::sync::Semaphore>,
    /// Kernel event bus — broadcast to all SSE/WebSocket subscribers.
    /// Capacity 256: events are small, subscribers should keep up.
    /// Lagging subscribers get BroadcastStreamRecvError::Lagged → skip.
    pub event_tx: tokio::sync::broadcast::Sender<KernelEvent>,
    /// Boot-time integrity verification: did the last stored verdict's BLAKE3 hash match?
    /// Set once at boot, exposed in /health for proprioception.
    pub chain_verified: AtomicBool,
    /// Latest proprioceptive snapshot — updated by probe scheduler (Ring 2).
    /// None until first probe cycle completes. Auth-gated in /health.
    pub environment: Arc<std::sync::RwLock<Option<crate::domain::probe::EnvironmentSnapshot>>>,
    /// Tracks TTL for dynamically registered Dogs (via POST /dogs/register).
    /// Config-based Dogs are permanent and not tracked here.
    /// RwLock: read on every heartbeat, write on register/expire.
    pub registered_dogs: Arc<std::sync::RwLock<HashMap<String, RegisteredDog>>>,
    /// D4: In-memory store for async judge jobs — progressive Dog arrival polling.
    pub judge_jobs: Arc<JudgeJobStore>,
    /// Self-model: expected system state loaded from backends.toml at boot.
    /// Compared against live roster in /health to detect missing Dogs.
    pub system_contract: Arc<std::sync::RwLock<crate::domain::contract::SystemContract>>,
}

/// Storage topology — exposed on authenticated /health for discoverability.
#[derive(Debug, Clone)]
pub struct StorageInfo {
    pub namespace: String,
    pub database: String,
}

/// F22: Cached DB readiness — avoids storage.ping() on every /ready call.
/// 30s TTL. Under load, /ready called every 5-10s by probes.
pub struct ReadyCache {
    ok: AtomicBool,
    checked_at: AtomicU64,
}

impl Default for ReadyCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadyCache {
    const TTL_SECS: u64 = 30;

    pub fn new() -> Self {
        Self {
            ok: AtomicBool::new(false),
            checked_at: AtomicU64::new(0),
        }
    }

    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Returns cached value if fresh, None if stale.
    pub fn get(&self) -> Option<bool> {
        let age = Self::now_secs().saturating_sub(self.checked_at.load(Ordering::Acquire));
        if age <= Self::TTL_SECS {
            Some(self.ok.load(Ordering::Acquire))
        } else {
            None
        }
    }

    pub fn set(&self, ok: bool) {
        self.ok.store(ok, Ordering::Release);
        self.checked_at.store(Self::now_secs(), Ordering::Release);
    }
}

impl std::fmt::Debug for ReadyCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadyCache")
            .field("ok", &self.ok.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("storage_info", &self.storage_info)
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .finish_non_exhaustive()
    }
}

impl AppState {
    pub fn storage_metrics(&self) -> Option<crate::domain::storage::StorageMetrics> {
        self.storage.metrics()
    }

    /// Centralized system health assessment (SSoT for REST and MCP).
    /// Computes status from Dogs, storage, probes, background tasks, and contract.
    pub async fn system_health(&self) -> crate::domain::health_gate::HealthAssessment {
        let judge = self.judge.load_full();
        let dog_health = judge.dog_health();
        let (healthy_dogs, total_dogs) =
            crate::domain::health_gate::count_healthy_dogs(&dog_health);

        let storage_ok = self.storage.ping().await.is_ok();
        let live_dog_ids = judge.dog_ids();

        let contract_guard = self
            .system_contract
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let contract_delta = contract_guard.assess(&live_dog_ids);

        let probes_degraded =
            crate::domain::probe::EnvironmentSnapshot::is_degraded(&self.environment);
        let stale_tasks = self.task_health.readiness_stale_tasks();

        crate::domain::health_gate::system_health_assessment_with_contract(
            healthy_dogs,
            total_dogs,
            storage_ok,
            probes_degraded,
            &stale_tasks,
            Some(&contract_delta),
        )
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

impl std::fmt::Debug for PerIpRateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerIpRateLimiter")
            .field("max_per_minute", &self.max_per_minute)
            .finish_non_exhaustive()
    }
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

/// POST /dogs/register — register a new Dog at runtime.
#[derive(Debug, Deserialize)]
pub struct RegisterDogRequest {
    /// Unique Dog ID (must not collide with existing).
    pub name: String,
    /// OpenAI-compatible base URL (e.g. "http://host:8080/v1").
    pub base_url: String,
    /// Model name for prompt routing.
    pub model: String,
    /// Optional API key for authenticated backends.
    pub api_key: Option<String>,
    /// Max context tokens (0 = unknown/unlimited). Default: 4096.
    #[serde(default = "default_context_size")]
    pub context_size: u32,
    /// Timeout in seconds. Default: 60.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_context_size() -> u32 {
    4096
}
fn default_timeout() -> u64 {
    60
}

#[derive(Debug, Serialize)]
pub struct RegisterDogResponse {
    pub dog_id: String,
    pub calibration: String,
    pub roster_size: usize,
}

/// Tracks TTL for Dogs registered via POST /dogs/register.
/// Config-based Dogs (backends.toml) are permanent and not tracked here.
#[derive(Debug)]
pub struct RegisteredDog {
    pub registered_at: std::time::Instant,
    pub last_heartbeat: std::time::Instant,
    pub ttl_secs: u64,
}

#[derive(Debug, Serialize)]
pub struct HeartbeatResponse {
    pub dog_id: String,
    pub status: String,
    pub ttl_remaining_secs: u64,
}

#[derive(Debug, Serialize)]
pub struct DeregisterResponse {
    pub dog_id: String,
    pub status: String,
    pub roster_size: usize,
}

#[derive(Debug, Deserialize)]
pub struct JudgeRequest {
    pub content: String,
    pub context: Option<String>,
    pub domain: Option<String>,
    /// Optional: evaluate with only these Dogs (by ID). If omitted, all Dogs are used.
    pub dogs: Option<Vec<String>>,
    /// Optional: disable crystal injection for A/B testing. Default: true.
    #[serde(default = "default_true")]
    pub crystals: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
pub struct JudgeResponse {
    pub verdict_id: String,
    pub domain: String,
    pub verdict: String,
    pub q_score: QScoreResponse,
    pub reasoning: ReasoningResponse,
    pub dogs_used: String,
    pub phi_max: f64,
    pub timestamp: String,
    pub dog_scores: Vec<DogScoreResponse>,
    /// Number of Dogs that contributed to this verdict (T9: transparency).
    /// Consumers can distinguish single-Dog from consensus.
    pub voter_count: usize,
    pub anomaly_detected: bool,
    pub max_disagreement: f64,
    pub anomaly_axiom: Option<String>,
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

#[derive(Debug, Clone, Serialize)]
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
    /// Raw scores before phi_bound — what the model actually produced.
    pub raw_fidelity: f64,
    pub raw_phi: f64,
    pub raw_verify: f64,
    pub raw_culture: f64,
    pub raw_burn: f64,
    pub raw_sovereignty: f64,
    pub reasoning: ReasoningResponse,
}

#[derive(Debug, Clone, Serialize)]
pub struct QScoreResponse {
    pub total: f64,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReasoningResponse {
    pub fidelity: String,
    pub phi: String,
    pub verify: String,
    pub culture: String,
    pub burn: String,
    pub sovereignty: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize)]
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
