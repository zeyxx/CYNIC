# CYNIC Probe System — Proprioception Spec

**Status**: Design validated, spec reviewed (iteration 1 fixes applied).
**Date**: 2026-03-30
**Approach**: C-direct (Absorb) — unified probe system, no parallel sensing.

## Problem

CYNIC's application layer is agnostic (StoragePort, ChatPort, EmbeddingPort). The infrastructure layer is not — hardcoded to systemd, Tailscale, SurrealDB, specific machines. The kernel has eyes (Dogs) but no body awareness. It cannot verify its own infrastructure health (VERIFY axiom gap), and the infra description doesn't match reality (FIDELITY gap).

Three sensing systems exist but are fragmented:
- `SystemMetricsPort` + `SysinfoMetrics` — CPU/RAM/disk (5min via introspection)
- `health_loop` — Dog HTTP reachability (30s)
- `introspection` — MAPE-K that mixes sensing with policy

## Design Principles

1. **Sensing separated from policy.** Probes sense. MAPE-K analyzes. Adaptation executes. (Netflix, Erlang, K8s all enforce this.)
2. **Denied is a status, not an error.** Permission denied = "I can't see this." The kernel continues.
3. **Per-probe intervals.** Dogs need 30s. Disk needs 5min. Backup needs 1h. One scheduler, individual clocks. (Telegraf pattern.)
4. **Baseline re-probed.** No stale knowledge. Periodic re-measurement of "normal." (Netflix minRTT pattern.)
5. **Zero new external dependencies for Phase 1.** `sysinfo` already in Cargo.toml. `netdev` deferred to Phase 3+.
6. **K1 compliant.** Zero `#[cfg]` in domain code. Platform-specific logic lives in infra/ adapters.
7. **Domain purity.** Zero `serde_json::Value` in domain types. Probe details are typed enums.

## Architecture

```
domain/probe.rs          — Probe trait, ProbeResult, ProbeDetails, EnvironmentSnapshot, NullProbe
domain/storage.rs        — extended: store/query infra_snapshot (default no-op)
  │
infra/probes/
  ├── mod.rs             — ProbeScheduler (runs probes at their intervals)
  ├── resource.rs        — ResourceProbe (CPU/RAM/disk via sysinfo)
  ├── backup.rs          — BackupProbe (stat backup dir, check freshness)
  ├── dog_health.rs      — DogHealthProbe (HTTP ping, circuit breakers) [Phase 2]
  ├── network.rs         — NetworkProbe (interfaces, peer latency) [Phase 3]
  └── os_capability.rs   — OsCapabilityProbe (cgroups, fd limits) [Phase 3]
  │
infra/tasks.rs           — spawn_probe_scheduler() background task
api/rest/health.rs       — extended: environment field in /health
introspection.rs         — simplified: consumes snapshots, no direct sensing
```

## Domain Contract

```rust
// domain/probe.rs — DOMAIN PURE, no #[cfg], no infra imports

use std::time::Duration;

/// A single aspect of the environment the kernel can sense.
/// Same discipline as Dogs: independent, timeout-bounded, graceful degradation.
#[async_trait]
pub trait Probe: Send + Sync {
    fn name(&self) -> &str;
    fn interval(&self) -> Duration;
    /// Sense one aspect. Permission denied, timeout, unreachable
    /// are all Ok(ProbeResult) with appropriate status.
    /// Err = internal bug only (logic bugs, invariant violations).
    /// OS errors (PermissionDenied, ENOENT, timeout) → Ok with Denied/Unavailable.
    async fn sense(&self) -> Result<ProbeResult, ProbeError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub name: String,
    pub status: ProbeStatus,
    pub details: ProbeDetails,
    pub duration_ms: u64,
    pub timestamp: String,  // RFC3339
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ProbeStatus {
    Ok,          // full data collected
    Degraded,    // partial data
    Unavailable, // cannot sense (no GPU, service down)
    Denied,      // permission denied — blind, not broken
}

/// Typed details per probe kind. No serde_json::Value — domain stays pure.
/// New probe types extend this enum (one variant per probe).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProbeDetails {
    Resource(ResourceDetails),
    Backup(BackupDetails),
    DogHealth(DogHealthDetails),   // Phase 2
    Network(NetworkDetails),        // Phase 3
    OsCapability(OsCapDetails),     // Phase 3
    Empty,                          // NullProbe and test doubles only. Real probes must use a typed variant.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDetails {
    pub cpu_usage_percent: Option<f32>,
    pub memory_used_gb: Option<f64>,
    pub memory_total_gb: Option<f64>,
    pub disk_available_gb: Option<f64>,
    pub disk_total_gb: Option<f64>,
    pub load_average_1m: Option<f64>,
    pub uptime_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupDetails {
    pub last_backup_age_hours: Option<f64>,
    pub last_backup_size_mb: Option<f64>,
    pub backup_count: Option<u32>,
    pub backup_dir: String,
}

// Phase 2+3 detail structs defined when implemented:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DogHealthDetails {
    pub dog_name: String,
    pub reachable: bool,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDetails {
    pub interfaces: Vec<NetInterfaceInfo>,
    pub peers: Vec<PeerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetInterfaceInfo {
    pub name: String,
    pub state: String,  // "up" | "down"
    pub ips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub name: String,
    pub address: String,
    pub reachable: bool,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsCapDetails {
    pub container: Option<String>,  // "docker" | "lxc" | "wsl2" | None
    pub memory_limit_gb: Option<f64>,
    pub cpu_quota: Option<f64>,
    pub fd_limit: Option<u64>,
    pub fd_used: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    pub timestamp: String,
    pub probes: Vec<ProbeResult>,
    pub overall: ProbeStatus, // worst status across all probes
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("probe internal error: {0}")]
    Internal(String),
}

// ── Null implementation (test double) ──

/// No-op probe for tests and environments where sensing is disabled.
pub struct NullProbe;

#[async_trait]
impl Probe for NullProbe {
    fn name(&self) -> &str { "null" }
    fn interval(&self) -> Duration { Duration::from_secs(3600) }
    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        Ok(ProbeResult {
            name: "null".into(),
            status: ProbeStatus::Unavailable,
            details: ProbeDetails::Empty,
            duration_ms: 0,
            timestamp: "1970-01-01T00:00:00Z".into(),
        })
    }
}
```

## StoragePort Extension

```rust
// Added to domain/storage.rs — default no-op implementations.
// NullStorage and existing test adapters continue to work unchanged.

async fn store_infra_snapshot(&self, _snap: &EnvironmentSnapshot) -> Result<(), StorageError> {
    Ok(()) // default: no-op
}

async fn list_infra_snapshots(&self, _hours: u32) -> Result<Vec<EnvironmentSnapshot>, StorageError> {
    Ok(vec![]) // default: empty
}

async fn cleanup_infra_snapshots(&self, _older_than_days: u32) -> Result<u64, StorageError> {
    Ok(0) // default: no-op
}
```

## ProbeScheduler

```rust
// infra/probes/mod.rs

pub struct ProbeScheduler {
    probes: Vec<ProbeSlot>,
}

struct ProbeSlot {
    probe: Arc<dyn Probe>,
    last_fired: Instant,
}

impl ProbeScheduler {
    pub fn new(probes: Vec<Arc<dyn Probe>>) -> Self { ... }

    /// Run one tick: fire all probes whose interval has elapsed.
    /// Fan-out in parallel (like Judge fans out to Dogs).
    /// Timeout per probe: 2x their interval, capped at 30s.
    /// Returns None if no probes were due this tick.
    pub async fn tick(&mut self) -> Option<EnvironmentSnapshot> { ... }
}
```

**AppState holds `Arc<RwLock<Option<EnvironmentSnapshot>>>` — not the scheduler itself.** The scheduler is owned by its background task. `/health` reads the latest snapshot from AppState. This keeps K2 compliant: no infra types leak into app state.

Scheduling loop in `spawn_probe_scheduler`:
- **First tick skipped** (consistent with all other background tasks: `interval.tick().await` before loop)
- Wakes every 10s (GCD of typical intervals)
- Checks each probe: `now - last_fired >= probe.interval()`
- Fires due probes in parallel via `join_all` with per-probe `tokio::time::timeout`
- Aggregates into `EnvironmentSnapshot`, writes to `AppState.environment`
- Persists to `infra_snapshot` table (best-effort, like verdict storage)
- Cleanup: every 100 ticks (~17min), delete snapshots older than 7 days
- Touches `TaskHealth::touch_probe_scheduler()`
- **Probe construction failure**: if a probe's `sense()` returns `Err(ProbeError::Internal)`, log ERROR, exclude from snapshot, continue with remaining probes. The scheduler does NOT crash.

## Storage Schema

```sql
-- New table in infra/surrealdb/schema.surql
DEFINE TABLE IF NOT EXISTS infra_snapshot SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS ts ON infra_snapshot TYPE datetime;
DEFINE FIELD IF NOT EXISTS overall ON infra_snapshot TYPE string;
DEFINE FIELD IF NOT EXISTS probes ON infra_snapshot TYPE array;
DEFINE FIELD IF NOT EXISTS probes.* ON infra_snapshot TYPE object;
DEFINE INDEX IF NOT EXISTS idx_infra_ts ON infra_snapshot FIELDS ts;
```

Retention: 7 days. Cleanup by the scheduler on each tick (same pattern as usage TTL cleanup).

## /health Extension

```json
{
  "status": "sovereign",
  "environment": {
    "timestamp": "2026-03-30T10:00:00Z",
    "overall": "ok",
    "probes": [
      {
        "name": "resource",
        "status": "ok",
        "details": { "Resource": { "cpu_usage_percent": 23.1, "memory_used_gb": 4.2, "memory_total_gb": 15.5, "disk_available_gb": 120, "disk_total_gb": 500, "load_average_1m": 0.82, "uptime_seconds": 86400 } },
        "duration_ms": 210
      },
      {
        "name": "backup",
        "status": "ok",
        "details": { "Backup": { "last_backup_age_hours": 6.2, "last_backup_size_mb": 42, "backup_count": 7, "backup_dir": "<configured_backup_dir>" } },
        "duration_ms": 2
      }
    ]
  },
  "dogs": [ "..." ],
  "background_tasks": [ "..." ]
}
```

Unauthenticated: `environment` omitted (same gating as dogs/tasks).

## Phase 1 — New Only (this session)

| Component | File | What |
|---|---|---|
| `Probe` trait | `domain/probe.rs` | Trait + ProbeResult + ProbeDetails + ProbeStatus + EnvironmentSnapshot + NullProbe |
| `ResourceProbe` | `infra/probes/resource.rs` | CPU/RAM/disk via sysinfo (reuses existing SysinfoMetrics logic) |
| `BackupProbe` | `infra/probes/backup.rs` | Stat backup dir, check age, check size |
| `ProbeScheduler` | `infra/probes/mod.rs` | Per-probe interval scheduling, parallel fan-out |
| `spawn_probe_scheduler` | `infra/tasks.rs` | Background task, wired in main.rs |
| `StoragePort` extension | `domain/storage.rs` | 3 new methods with default no-op |
| `SurrealHttpStorage` impl | `storage/surreal.rs` | store/query/cleanup infra_snapshot |
| `infra_snapshot` table | `infra/surrealdb/schema.surql` | Schema definition |
| `/health` extension | `api/rest/health.rs` | `environment` field from AppState |
| `AppState` | `api/rest/mod.rs` | `environment: Arc<RwLock<Option<EnvironmentSnapshot>>>` |
| `TaskHealth` | `infra/task_health.rs` | `touch_probe_scheduler()` field + snapshot entry |
| Tests | Per-file | NullProbe, ResourceProbe mock, BackupProbe mock, scheduler tick, storage round-trip |

**Does NOT touch:** `health_loop.rs`, `system_metrics.rs`, `introspection.rs`. They continue unchanged. Phase 2-4 absorb them.

## Phase 2 — Absorb health_loop (future session)

- `DogHealthProbe` wraps Dog HTTP pinging + circuit breaker updates
- **Critical**: `DogHealthProbe.sense()` must side-effect the circuit breaker immediately on each ping, not wait for snapshot aggregation. Circuit breaker responsiveness must not degrade.
- `health_loop.rs` deleted
- ProbeScheduler runs DogHealthProbe at 30s interval

## Phase 3 — Absorb SystemMetricsPort (future session)

- `ResourceProbe` becomes the sole resource sensor
- `SystemMetricsPort` trait deleted, `SysinfoMetrics` adapter deleted
- `introspection.rs` reads from `AppState.environment` instead of calling SystemMetricsPort
- `NetworkProbe` added (requires `netdev` crate — new dependency decision via cynic-empirical)
- `OsCapabilityProbe` added (cgroups, fd limits — file reads, no crate)

## Phase 4 — Introspection simplification (future session)

- Introspection becomes analysis-only: reads `EnvironmentSnapshot`, produces `Alert`
- Trending: queries `infra_snapshot` table for 24h window, detects drift
- Adaptation rules: bounded, per-resource (disk >90% → reduce observation retention, etc.)
- Sensing/policy fully separated (production principle #1)

## Sensing Layers (foundation supports all, implement progressively)

| Layer | What | Phase | Cross-platform |
|---|---|---|---|
| 1. Process | PID, RAM, fd, threads, uptime | 1 (ResourceProbe) | sysinfo |
| 2. Machine | CPU total, disk, GPU | 1 (ResourceProbe) | sysinfo |
| 3. Peers | Dog reachability, latency | 2 (DogHealthProbe) | HTTP |
| 4. Network | Interfaces, IPs, VLANs, routes | 3 (NetworkProbe) | netdev + fallback |
| 5. OS Advanced | cgroups, namespaces, capabilities | 3 (OsCapabilityProbe) | file reads + graceful Denied |
| 6. Fleet | Cross-node state, distributed sensing | Future | Requires agent protocol |

## Consumer Pipeline

```
Probes → EnvironmentSnapshot
  → SurrealDB (persist for trending)
  → AppState.environment (in-memory for /health)
  → MAPE-K introspection (analyze trends, produce Alerts) [Phase 4]
    → Alerts → Crystal pipeline (epistemic memory)
    → Alerts → Event bus (SSE, webhooks)
    → Alerts → Adaptation rules (bounded parameter adjustment)
  → MCP cynic_health (agents read environment)
```

## Falsification Criteria

This design should be REJECTED if:
1. The ProbeScheduler adds >50ms latency to any existing background task
2. `infra_snapshot` table grows >100MB in 7 days (indicates schema bloat)
3. Phase 2 absorption of health_loop degrades circuit breaker response time by >5s
4. Adding a new probe requires modifying more than 2 files: the new `infra/probes/<name>.rs` + `main.rs` wiring. (`ProbeDetails` enum variant is a third file but is the domain contract, not implementation leakage. `TaskHealth` is NOT modified per-probe — the scheduler touches once per tick, not per-probe.)
5. The kernel with all probes returning `Denied` (container, no permissions) behaves identically to kernel without probe system — verified by running in a restricted container with seccomp dropping `stat`/`read`, confirming `/health` returns `environment.overall: "denied"` not HTTP 500.
