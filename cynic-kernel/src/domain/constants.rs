//! Operational constants — single source of truth.
//!
//! Domain-level constants that were previously scattered as magic numbers
//! across runtime_loops.rs, main.rs, pipeline/mod.rs, and api/ handlers.
//! NOT config — these are compile-time constants with semantic names.

use std::time::Duration;

// ── Network defaults ──────────────────────────────────────────

/// Default kernel REST address (bind + self-discovery).
/// Used by main.rs, runtime_loops.rs (discovery), backends (embedding, summarizer).
pub const DEFAULT_REST_ADDR: &str = "127.0.0.1:3030";

/// Default HTTP timeout for outbound calls (model probes, registration).
pub const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for Dog registration POST requests.
pub const REGISTRATION_TIMEOUT: Duration = Duration::from_secs(10);

/// Timeout for Slack webhook delivery.
pub const SLACK_ALERT_TIMEOUT: Duration = Duration::from_secs(3);

// ── Background loop intervals ─────────────────────────────────

/// Dog TTL checker — remove stale Dogs from roster.
pub const DOG_TTL_CHECK_INTERVAL: Duration = Duration::from_secs(30);

/// Dog heartbeat — refresh TTL for registered Dogs.
pub const DOG_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(40);

/// Service discovery — probe fleet for new Dogs.
pub const DISCOVERY_INTERVAL: Duration = Duration::from_secs(60);

/// Crystal immune system — re-judge oldest crystallized knowledge.
pub const CRYSTAL_CHALLENGE_INTERVAL: Duration = Duration::from_secs(300);

/// Per-crystal re-judge timeout.
/// 60s: gemma-4b-core (CPU) averages 22s, needs headroom for tail latency + DB.
pub const CRYSTAL_CHALLENGE_TIMEOUT: Duration = Duration::from_secs(60);

/// Event consumer idle liveness tick.
pub const EVENT_LIVENESS_INTERVAL: Duration = Duration::from_secs(60);

/// Probe scheduler polling interval.
pub const PROBE_SCHEDULER_INTERVAL: Duration = Duration::from_secs(10);

// ── Capacity limits ───────────────────────────────────────────

/// Max stimulus content length (bytes).
pub const MAX_CONTENT_LEN: usize = 4000;

/// Max stimulus context length (bytes).
pub const MAX_CONTEXT_LEN: usize = 2000;

/// Max concurrent judge jobs.
pub const MAX_JUDGE_JOBS: usize = 100;

/// Judge job TTL before auto-expiry.
pub const JUDGE_JOB_TTL: Duration = Duration::from_secs(300);

/// Batch size for crystal challenge queries.
pub const CRYSTAL_CHALLENGE_BATCH: u32 = 100;

/// Nightshift — autonomous dev judgment loop.
pub const NIGHTSHIFT_INTERVAL: Duration = Duration::from_secs(4 * 3600); // 4 hours
/// Per-commit judgment timeout (includes all Dogs).
pub const NIGHTSHIFT_COMMIT_TIMEOUT: Duration = Duration::from_secs(300); // 5 min
/// Git lookback window for nightshift commit discovery.
/// Must be a git-accepted approxidate ("24 hours ago" / "1 day ago"), NOT a
/// shorthand like "24h" which git silently returns empty for. Observed
/// 2026-04-17: "24h" → 0 commits while "24 hours ago" → 3 commits for the
/// same repo state. Silent fail mode of `git log --since`.
pub const NIGHTSHIFT_GIT_LOOKBACK: &str = "24 hours ago";

// ── Concurrency bounds ────────────────────────────────────────

/// Background fire-and-forget task semaphore.
pub const BG_SEMAPHORE_PERMITS: usize = 64;

/// SSE connection semaphore.
pub const SSE_SEMAPHORE_PERMITS: usize = 32;

/// Heartbeat TTL margin — refresh if remaining TTL < this (seconds).
pub const HEARTBEAT_TTL_MARGIN: u64 = 60;

/// Default Dog registration TTL (seconds) in discovery payloads.
pub const DEFAULT_REGISTRATION_TTL: u64 = 90;

// ── Budget calibration ─────────────────────────────────────

/// Minimum completion budget — prevents thinking model death spirals.
/// Thinking models (Gemma 4) consume tokens on reasoning before producing JSON.
/// Without a floor, the budget calibrates on truncated outputs and collapses.
/// 768: covers ~450 thinking tokens + ~300 content tokens with margin.
pub const MIN_COMPLETION_BUDGET: u32 = 768;
