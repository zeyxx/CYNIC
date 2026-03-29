# Proprioception + MCP Write Path — v0.7.4

*"Un systeme immunitaire sans systeme nerveux."*

Date: 2026-03-28
Session: S2 (G17 proprioception) + S5 (G4 MCP write path) fused
Version target: v0.7.4 (v0.8 gates unchanged)

---

## Problem

The kernel judges external content but cannot sense its own body. Introspection exists (4 checks, 5-min tick) but self-observation crystals never persist — T8 quorum gate rejects `voter_count=0`. The binary cannot report its own version. MCP has 10 tools, all read/judge — agents cannot write observations back.

### Evidence from code

- `introspection.rs:106` calls `observe_crystal(voter_count=0)` — rejected by quorum gate (T8)
- `main.rs` has `--reset` and `--mcp` only — no `--version`
- `api/mcp/mod.rs` has 10 tools: judge, health, verdicts, crystals, infer, audit, coord_register/claim/claim_batch/release/who — zero write tools
- `domain/metrics.rs` has pipeline counters only — zero system metrics
- `probe/hardware.rs` uses `sysinfo` at boot for 3 static fields — no runtime metrics
- `api/rest/observe.rs::infer_domain()` is domain logic trapped in an API adapter (K5 violation)

---

## Crystallized Truths (from analysis)

| T# | Truth | Confidence | Design Impact |
|----|-------|-----------|---------------|
| T1 | v0.8.0 is a gate milestone, not a feature container. Proprioception doesn't close any v0.8 gate. | 58% | Tag as v0.7.4. v0.8 reserved for G1+G2+G3. |
| T2 | T8 and --version are proprioception deliverables, not pre-existing debt. | 61% | Include as L1 and L2. |
| T3 | Prioritize by causal dependency, not by category (debt vs feature). | 56% | T8 fix before probes. Port trait before adapter. |
| T4 | Findings tracker consolidation is a v0.8 G1 prerequisite, not this session. | 52% | Track, don't block. |
| T5 | Proprioception inverts the debt trajectory — the kernel detects its own pain. | 48% | This is the mechanism that makes future debt self-revealing. |

---

## Approach Selection

### Chosen: Reuse Existing Pipeline (Approach A)

Infra observations follow the same path as judgment observations. No new crystallization pipeline.

```
Periodic probes --> store_observation(domain="infra") --> analytics
       |
       +-> introspection detects anomaly --> pipeline::run(Stimulus) --> Dogs judge --> crystal
```

### Rejected: Direct Infra Crystallization (Approach B)

Bypasses Dogs. Violates trust model. Creates parallel crystallization path (violates K3). Same shortcut that caused F15 and Attack Chain 1.

### Deferred: Full Sensing Layer (Approach C)

Over-engineering for 4-5 probe types. Rule 17 (bugs before abstractions). Good candidate for v0.9 if probes multiply.

---

## Deliverables

### L1. --version flag

**File:** `main.rs`
**Change:** 4 lines before tracing init, after `--reset`/`--mcp` flag parsing (lines 13-14).

```rust
if std::env::args().any(|a| a == "--version") {
    println!("cynic-kernel {}", env!("CARGO_PKG_VERSION"));
    return Ok(());
}
```

**Placement:** Before tracing init — `--version` needs no logging, no boot sequence. After the existing `force_reprobe`/`mcp_mode` arg parsing at lines 13-14.

**Why first:** Binary self-identification is the simplest proprioception. Unblocks observability.

---

### L2. T8 fix — introspection becomes pipeline client

**Root cause:** `introspection.rs` calls `observe_crystal(voter_count=0)` directly. Quorum gate (T8) requires `voter_count >= 2`. Self-observation crystals never persist.

**Fix:** Introspection submits anomalies to `pipeline::run` instead. Dogs evaluate. Real voter_count. Quorum gate works naturally.

| File | Change |
|------|--------|
| `infra/tasks.rs` | `spawn_introspection` receives 4 additional Arcs: `Judge`, `EmbeddingPort`, `VerdictCache`, `Mutex<DogUsageTracker>`. Timeout raised from 30s to 180s (Dogs need 30-120s per evaluation). |
| `main.rs` | Pass the 4 Arcs (already exist as local variables at call site) |
| `introspection.rs` | `analyze()` receives the individual Arcs, constructs `PipelineDeps<'_>` locally from references at call time (borrows cannot cross tokio::spawn — Arcs are owned by the task, PipelineDeps borrows from them inside the tick). |
| `pipeline.rs` | In `observe_crystal_for_verdict`: skip `store_crystal_embedding` when `domain == "cynic-internal"`. Crystal persists (state machine works) but is excluded from KNN index — prevents cross-domain contamination. This matches the existing anti-contamination intent documented in `introspection.rs` module comment. |

**Key design decisions:**

1. **Anti-contamination via KNN exclusion (fixes B3).** `inject_crystals=false` prevents reading existing crystals into Dog prompts, but does NOT prevent the verdict's crystal from being written + embedded. The real lever: skip `store_crystal_embedding` for `domain="cynic-internal"`. Crystal exists in storage (queryable by domain listing), but invisible to semantic search. No self-referential noise in the KNN index.

2. **Lifetime correctness (fixes B2).** `spawn_introspection` owns `Arc<Judge>`, `Arc<dyn EmbeddingPort>`, `Arc<VerdictCache>`, `Arc<Mutex<DogUsageTracker>>`. Inside the tick loop, `analyze()` constructs `PipelineDeps<'_>` from `&*arc` references. The struct borrows; the task owns. No lifetime violation.

3. **Timeout budget (fixes B1).** Outer timeout raised to 180s. Dogs can take 30-120s each. `dogs_filter: Some(&["deterministic-dog", "gemini-flash"])` limits to 2 fast Dogs — DeterministicDog is instant, Gemini Flash is ~1s. voter_count = 2, passes quorum, finishes in <5s typical. 180s is the safety margin for degraded network.

4. **Storm consolidation (fixes C5).** All alerts from a single tick are concatenated into ONE `pipeline::run` call (not one per alert). Content bounded to 2000 chars. Single Dog evaluation per tick regardless of alert count.

**Verification:**
- Integration test: mock storage down → introspection → assert crystal persisted with voter_count >= 2
- Integration test: assert `cynic-internal` crystal does NOT appear in `search_crystals_semantic` results for unrelated queries (anti-contamination invariant)

---

### L3. SystemMetrics port trait

**New file:** `domain/system_metrics.rs`

```rust
use async_trait::async_trait;
use std::fmt;

#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub cpu_usage_percent: f64,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub disk_available_gb: f64,
    pub disk_total_gb: f64,
    pub load_average_1m: f64,
    pub uptime_seconds: u64,
    pub created_at: String,
}

#[derive(Debug)]
pub struct SystemMetricsError(pub String);

impl fmt::Display for SystemMetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "system metrics error: {}", self.0)
    }
}

impl std::error::Error for SystemMetricsError {}

#[async_trait]
pub trait SystemMetricsPort: Send + Sync {
    async fn snapshot(&self) -> Result<SystemSnapshot, SystemMetricsError>;
}
```

**Rules satisfied:**
- K1: Zero `#[cfg]` in domain. Port doesn't know sysinfo exists.
- K4: Unique trait name (`SystemMetricsPort`).
- K7: `Display` implies `Error` on `SystemMetricsError`.
- Rule 8 (port contracts first): Trait before adapter.

**Also add to `domain/mod.rs`:** `pub mod system_metrics;`

---

### L4. sysinfo adapter + introspection broadened

#### L4a. Adapter

**New file:** `infra/system_metrics.rs`

Implements `SystemMetricsPort` using sysinfo 0.37 (already a dependency):

- **Per-tick `System::new()` + targeted refresh** (not persistent instance). `System::new()` is nearly free (no process enumeration). `refresh_memory()` + `refresh_cpu_specifics()` are cheap targeted calls. This avoids the mutex soundness complexity of a persistent `System` — same proven pattern as `probe/hardware.rs`.
- `Disks::new_with_refreshed_list()` for disk space (cross-platform)
- `System::load_average()` for 1-min load (returns 0.0 on Windows — acceptable)
- `System::uptime()` for uptime seconds
- All inside `spawn_blocking` to avoid blocking async runtime (sysinfo calls are synchronous)

**Also wire in `infra/mod.rs`.**

**Note:** First tick after boot returns `cpu_usage=0.0` (sysinfo needs two samples). Introspection skips CPU alerting when `cpu_usage_percent == 0.0`.

#### L4b. Introspection broadened

**File:** `introspection.rs`

`analyze()` receives `&dyn SystemMetricsPort` in addition to existing deps.

New checks (alongside existing 4):

| Check | Condition | Severity |
|-------|-----------|----------|
| `memory_pressure` | used/total > 90% | warning; critical if > 95% |
| `cpu_sustained` | cpu_usage > 80% | warning (single sample — no history yet) |
| `disk_low` | available/total < 10% | warning; critical if < 5% |

Anomalies go through `pipeline::run(domain="cynic-internal")` per L2 fix.

Raw snapshot stored as observation:
```
store_observation(Observation {
    tool: "self-probe",
    target: hostname or "localhost",
    domain: "infra",
    status: "ok" | "warning" | "critical",
    context: "cpu:42.1% mem:11.2/15.5GB disk:120/500GB load:0.82 up:3d",
    ...
})
```

**Context format:** Compact key:value string, not JSON. Fits within 200-char truncation limit. Fields: cpu usage%, mem used/total, disk avail/total, load 1m, uptime shorthand.

#### L4c. Wiring in main.rs

```rust
let system_metrics: Arc<dyn SystemMetricsPort> = Arc::new(SysinfoMetrics::new());
// Pass to spawn_introspection alongside other deps
```

Boot order: system_metrics created in Ring 0 (alongside probe), passed to Ring 2 tasks.

---

### L5. MCP `cynic_observe` + domain extraction

#### L5a. Extract `infer_domain` to domain

**From:** `api/rest/observe.rs::infer_domain(target)`
**To:** `domain/ccm.rs::infer_domain(target)` (it's domain logic — classifying content by extension/pattern)

Add infra mapping:
- `.service`, `.timer` extensions → `"infra"`
- Targets matching `cynic-*`, `llama-server`, `surrealdb` → `"infra"`
- `self-probe` tool → `"infra"` (regardless of target)

REST handler now calls `domain::ccm::infer_domain()`.

**Test migration:** The ~10 unit tests for `infer_domain` in `api/rest/observe.rs` move with the function to `domain/ccm.rs`. New tests added for infra mappings (`.service`→infra, `cynic-*`→infra, `self-probe`→infra).

#### L5b. MCP tool

**File:** `api/mcp/mod.rs`

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ObserveParams {
    #[schemars(description = "Tool or action name (1-64 chars)")]
    pub tool: String,
    #[schemars(description = "Target file, resource, or entity")]
    pub target: Option<String>,
    #[schemars(description = "Domain classification (auto-inferred if omitted)")]
    pub domain: Option<String>,
    #[schemars(description = "Status: ok, warning, error")]
    pub status: Option<String>,
    #[schemars(description = "Additional context (max 200 chars)")]
    pub context: Option<String>,
    #[schemars(description = "Project identifier")]
    pub project: Option<String>,
    #[schemars(description = "Agent identifier for session tracking")]
    pub agent_id: Option<String>,
    #[schemars(description = "Session identifier for CCM aggregation")]
    pub session_id: Option<String>,
}

#[tool(name = "cynic_observe")]
async fn cynic_observe(&self, params: ObserveParams) -> Result<CallToolResult, McpError> {
    self.rate_limit.check_other()?;  // 30/min bucket
    // validate tool length (1-64)
    // infer domain via domain::ccm::infer_domain
    // build Observation
    // tokio::spawn with timeout(5s) for storage.store_observation (fire-and-forget)
    // audit log
    // return {"status": "observed"}
}
```

**Parity with REST:** `agent_id` and `session_id` included (REST has both). Without them, MCP observations break session-level CCM aggregation.

**Rate limiting:** `check_other()` (30/min) at function entry. Fire-and-forget via `tokio::spawn` with `timeout(5s)` — no `bg_semaphore` (REST-specific). Bounded by rate limiter + tokio task pool.

Same validation, same domain inference, same storage call as REST. K3 satisfied.

---

## What Does NOT Change

- No new SurrealDB tables (observation + crystal tables sufficient)
- No new crystallization path (everything through pipeline::run)
- No new Cargo dependency (sysinfo 0.37 already present)
- No change to Dogs, scoring, or crystal injection format
- No `/test-chess` required (no scoring/prompt/crystal injection changes)
- No change to crystal lifecycle state machine

## Execution Order

```
L1 (--version)  -->  L2 (T8 fix)  -->  L3 (port trait)  -->  L4 (adapter+introspection)  -->  L5 (MCP observe)
    4 lines          wiring             domain pure            infra + integration              API surface
```

Each deliverable followed by `/build`. Final: bump Cargo.toml to 0.7.4, tag, `/deploy`.

## Testing Strategy

| Deliverable | Test |
|-------------|------|
| L1 | `cargo run -- --version` outputs version string |
| L2 | Integration test: mock storage down → introspection → assert crystal persisted with voter_count >= 2 |
| L3 | Compile-time: port trait compiles with no external deps. Unit test: SystemSnapshot fields. |
| L4 | Unit test: SysinfoMetrics::snapshot() returns sane values. Integration: threshold breach → Anomaly event emitted. |
| L5 | Unit test: cynic_observe validates inputs, infers domain. Integration: MCP observe → observation in storage. |

## Risk Assessment

| Risk | Mitigation |
|------|-----------|
| Dogs consume tokens on self-observation | `dogs_filter: Some(&["deterministic-dog", "gemini-flash"])` — 2 fast Dogs, no sovereign LLM cost. voter_count=2 passes quorum. |
| Introspection storms (many anomalies at once) | All alerts consolidated into ONE pipeline::run call per tick. Content concatenated, bounded to 2000 chars. |
| sysinfo::System thread safety | Per-tick `System::new()` inside `spawn_blocking`. No shared state. Same pattern as `probe/hardware.rs`. |
| cpu_usage requires 2 samples with delay | First tick returns 0.0 (sysinfo behavior). Introspection skips CPU alerting when value is 0.0. |
| Cross-domain crystal contamination | `cynic-internal` crystals excluded from KNN index (no embedding stored). Visible by domain listing only. |

## Review Fixes Applied

Spec reviewed by code-reviewer agent. 3 blockers, 5 concerns, 4 suggestions found and resolved:

| ID | Severity | Fix Applied |
|----|----------|-------------|
| B1 | Blocker | Timeout raised 30s→180s. dogs_filter limits to 2 fast Dogs. |
| B2 | Blocker | Arcs owned by task, PipelineDeps<'_> constructed locally from references. |
| B3 | Blocker | Anti-contamination via KNN exclusion (skip store_crystal_embedding for cynic-internal). |
| C1 | Concern | Rate limit = check_other(). Fire-and-forget via tokio::spawn + timeout(5s). |
| C2 | Concern | Tests travel with infer_domain. New infra mapping tests specified. |
| C3 | Concern | Per-tick System::new() instead of persistent mutex. No soundness issue. |
| C4 | Concern | timestamp→created_at (matches codebase convention). |
| C5 | Concern | Storm consolidation: one pipeline::run per tick with concatenated alerts. |
| S2 | Suggestion | agent_id + session_id added to ObserveParams (REST parity). |

## Future (not this session)

- Service process detection (is surrealdb running? is llama-server responding?)
- Network path probing (Tailscale peer latency, SPOF detection)
- 30s probe granularity (separate task, not introspection tick)
- History-aware thresholds (CPU > 80% sustained over 3 ticks, not single sample)
- Persistent System instance with proper lock management (if per-tick new() becomes a bottleneck)
