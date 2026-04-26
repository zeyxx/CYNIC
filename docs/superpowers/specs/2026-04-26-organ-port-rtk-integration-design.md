# OrganPort + RTK Integration — Design Spec

> **RTK becomes the first organ with its own DB in the CYNIC organism. This is not a tool integration — it's the cobaye for the organ-data-sovereignty pattern that every future organ will follow.**

Date: 2026-04-26
Status: Draft
Epistemic: Design validated through empirical challenge (OTel data model, Rust port patterns). Gemini contradicteur unavailable (quota). Confidence: 0.55 (inferred).
Spec review: 6 issues found, 3 adopted, 1 rejected, 2 adjusted. See §10.

---

## 1. Vision & Principles

### Context

RTK (Rust Token Killer) is an open-source CLI proxy (rtk-ai/rtk) that saves tokens in Claude Code interactions.

**Upstream state (observed 2026-04-26):**
- Latest stable: **v0.37.2** (2026-04-20). Dev: v0.38.0-rc.177 (daily RC cadence).
- 36K stars, 2189 forks, Apache 2.0 license. Very active project.
- Local install: **v0.30.1 — 7 minor versions behind.** Upgrade required before any implementation.
- No upstream export/metrics/API feature exists or is planned — our contribution would be novel.
- Project-local `.rtk/filters.toml` supported natively via `rtk trust` / `rtk untrust` / `rtk verify`.

**Local stats (observed on v0.30.1):**
- 15,868 commands processed
- 62.7M tokens saved (87.1% average savings)
- Data stored in `~/.local/share/rtk/history.db` (SQLite, 6.4M)
- Integrated via Claude Code hook (`rtk-rewrite.sh`)

**SQLite schema (observed on v0.30.1 — must be re-verified after upgrade to v0.37+):**
```sql
CREATE TABLE commands (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    original_cmd TEXT NOT NULL,
    rtk_cmd TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    saved_tokens INTEGER NOT NULL,
    savings_pct REAL NOT NULL,
    exec_time_ms INTEGER DEFAULT 0,
    project_path TEXT DEFAULT ''
);
CREATE TABLE parse_failures (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    raw_command TEXT NOT NULL,
    error_message TEXT NOT NULL,
    fallback_succeeded INTEGER NOT NULL DEFAULT 0
);
```

CYNIC has no metabolic awareness today — no token cost tracking, no BURN metrics, no resource accounting. The organism taxonomy (v2) identifies a "resource accounting" gap in the meta-cortex observer.

### What This Is

RTK becomes the **first organ with its own personal database** in the CYNIC organism. Simultaneously, Organ X (Hermes social perception) approaches with the same pattern (own data store at `~/.cynic/organs/hermes/x/`). Together they are the first two patients of the future Soma orchestrator.

### What This Is NOT

- Not a monitoring system (we don't build Prometheus)
- Not a Soma implementation (Soma emerges from observing these organs)
- Not a dashboard project (no consumer identified yet — K15)

### Principles (DATA-CENTRIC / CHAOS→MATRIX)

1. **Observe before structure.** We create a reader and observe what emerges. No `/metabolism` endpoint, no Soma module, no dashboard on day 1.

2. **Each organ owns its data.** The kernel never migrates, modifies, or controls `history.db`. It reads in read-only mode. The contract = the SQLite schema as-is. If RTK changes its schema, RTK carries the migration — the kernel adapts.

3. **The fork is a vehicle, not a divergence.** We fork RTK, iterate (CYNIC filters + export features), PR upstream what's generic. CYNIC-specific filters stay local.

4. **The 5 scalability aspects emerge from usage, not from abstract design:**
   - Data sovereignty (who owns what)
   - Discovery & registration (how the kernel finds organs)
   - Contention & isolation (concurrent readers/writers)
   - Schema evolution (who carries migrations)
   - Lifecycle (start/stop/crash/stale)

5. **This pattern enables faster adoption of external tools.** Instead of rebuilding what exists, we wrap open-source tools as organs — same interface, sovereign data, zero vendor lock-in.

---

## 2. Architecture

### OrganPort Trait (domain layer)

Designed to carry N organs with N functions and N states, with zero kernel code changes per new organ. Aligned with existing port patterns (StoragePort, BackendPort — all async, all Send+Sync, all with dedicated error types).

Challenged against OpenTelemetry data model: we adopt MetricKind (Counter/Gauge) for rate computation (first identified use case: tokens/hour). We skip OTel labels/attributes and Histogram types until a consumer demands them.

```rust
// domain/organ.rs

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::time::Duration;

#[async_trait]
pub trait OrganPort: Send + Sync {
    /// Stable organ identity (e.g. "rtk", "hermes-x")
    fn name(&self) -> &str;

    /// Organ liveness — Degraded/Dead carry a reason string
    async fn health(&self) -> OrganHealth;

    /// Age of the organ's most recent meaningful data.
    /// Semantics are organ-defined: RTK = last command timestamp,
    /// Hermes = last tweet captured. Not the same as snapshot time.
    async fn freshness(&self) -> Result<Duration, OrganError>;

    /// Timestamped bag of typed metrics. Caller can compute rates
    /// from consecutive Counter snapshots via delta/Δt.
    async fn snapshot(&self) -> Result<OrganSnapshot, OrganError>;
}

pub struct OrganSnapshot {
    pub taken_at: DateTime<Utc>,
    pub metrics: Vec<Metric>,
}

pub struct Metric {
    pub key: String,
    pub value: MetricValue,
    pub kind: MetricKind,
    pub unit: Option<String>,
}

pub enum MetricValue {
    F64(f64),
    I64(i64),
    Str(String),
    Bool(bool),
}

/// Counter = monotonically increasing (delta/time = rate).
/// Gauge = point-in-time snapshot (current value).
pub enum MetricKind {
    Counter,
    Gauge,
}

pub enum OrganHealth {
    Alive,
    Degraded { reason: String },
    Dead { reason: String },
}

#[derive(Debug)]
pub enum OrganError {
    /// Data source not found (DB missing, file absent)
    Unavailable(String),
    /// Data source found but read failed (IO, parse, lock timeout)
    ReadFailed(String),
}

impl std::fmt::Display for OrganError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(msg) => write!(f, "organ unavailable: {msg}"),
            Self::ReadFailed(msg) => write!(f, "organ read failed: {msg}"),
        }
    }
}

impl std::error::Error for OrganError {}
```

### Design Decisions Log

| Decision | Chosen | Alternatives considered | Why |
|----------|--------|------------------------|-----|
| `Vec<Metric>` bag vs typed structs per organ | Generic bag | Per-organ snapshot enum | Scales to N without kernel changes |
| MetricKind (Counter/Gauge) | Adopt day 1 | Skip, add later | Rate computation is first use case |
| OTel labels/attributes | Skip | HashMap<String,String> on Metric | No consumer filters by dimension yet |
| OTel Histogram/Summary | Skip | Full OTel type set | No distribution analysis use case yet |
| Health with reason | Adopt | Simple enum (Alive/Degraded/Dead) | Every past incident needed a reason |
| Async trait | Adopt | Sync | All existing ports are async; future organs may need network |
| Freshness on trait | Adopt | Derive from snapshot timestamp | Organ-defined semantics (RTK ≠ Hermes) |
| Snapshot-level timestamp | Adopt | Per-metric timestamp | Metrics are contemporaneous within one read |

### RtkReader — First Implementation

```
cynic-kernel/src/organs/
├── mod.rs          -- pub mod rtk; + organ registry (Vec<Box<dyn OrganPort>>)
└── rtk.rs          -- RtkReader: impl OrganPort
```

```rust
// organs/rtk.rs

pub struct RtkReader {
    db_path: PathBuf,      // ~/.local/share/rtk/history.db
    project_root: PathBuf, // git rev-parse --show-toplevel (no hardcoded paths — R1)
}

impl RtkReader {
    pub fn new(db_path: PathBuf, project_root: PathBuf) -> Self {
        Self { db_path, project_root }
    }

    /// Open DB read-only with 1s busy timeout.
    /// IMPORTANT: All calls to this must be wrapped in tokio::task::spawn_blocking()
    /// because rusqlite is synchronous and will block the async executor.
    fn open_db(&self) -> Result<Connection, OrganError> {
        // SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX
        // busy_timeout(1000) — if RTK is writing, wait 1s then fail gracefully
    }
}

#[async_trait]
impl OrganPort for RtkReader {
    fn name(&self) -> &str { "rtk" }

    async fn health(&self) -> OrganHealth {
        // DB exists + readable → Alive
        // DB exists + locked after timeout → Degraded { reason: "db locked" }
        // DB missing → Dead { reason: "history.db not found" }
    }

    async fn freshness(&self) -> Result<Duration, OrganError> {
        // SELECT MAX(timestamp) FROM commands
        // Parse ISO 8601 → Duration::since(now)
    }

    async fn snapshot(&self) -> Result<OrganSnapshot, OrganError> {
        // Single aggregation query, project-filtered:
        //
        // SELECT COUNT(*) as commands_total,
        //        SUM(input_tokens) as tokens_input,
        //        SUM(output_tokens) as tokens_output,
        //        SUM(saved_tokens) as tokens_saved,
        //        AVG(savings_pct) as savings_pct,
        //        SUM(exec_time_ms) as exec_time_ms
        // FROM commands
        // WHERE project_path LIKE ? (bound to self.project_root — R1: no hardcoded paths)
        //
        // + SELECT COUNT(*) FROM parse_failures
        //
        // Returns 7 metrics:
        //   commands_total    (Counter, "count")
        //   tokens_input      (Counter, "tokens")
        //   tokens_output     (Counter, "tokens")
        //   tokens_saved      (Counter, "tokens")
        //   savings_pct       (Gauge,   "%")
        //   exec_time_total   (Counter, "ms")
        //   parse_failures    (Counter, "count")
    }
}
```

### Organ Registry (startup)

```rust
// organs/mod.rs

pub fn build_organ_registry() -> Vec<Box<dyn OrganPort>> {
    let mut organs: Vec<Box<dyn OrganPort>> = Vec::new();

    // RTK — best-effort, non-fatal if missing
    let rtk_db = dirs::data_local_dir()
        .unwrap_or_default()
        .join("rtk/history.db");
    if rtk_db.exists() {
        organs.push(Box::new(rtk::RtkReader::new(rtk_db)));
    }

    // Hermes X — added when organ ships
    // organs.push(Box::new(hermes_x::HermesXReader::new(...)));

    organs
}
```

Static at startup. No dynamic discovery (N=2, static suffices). The registry lives in `AppState` alongside `StoragePort`, `BackendPort`, etc.

---

## 3. Filters TOML (Chantier A — parallel, zero RTK code)

```toml
# CYNIC/.rtk/filters.toml
schema_version = 1

[filters.cargo-build]
description = "Strip Compiling/Downloading noise from cargo build"
match_command = "^cargo build"
strip_lines_matching = ["^\\s*Compiling ", "^\\s*Downloading ", "^\\s*Downloaded "]
max_lines = 50

[filters.make-check]
description = "Keep verdict + errors from make check"
match_command = "^make check"
strip_lines_matching = ["^\\s*Compiling ", "^\\s*Finished ", "^\\s*Running "]
max_lines = 80

[filters.cargo-test]
description = "Strip passing tests, keep failures"
match_command = "^cargo test"
strip_lines_matching = ["^test .* ok$", "^\\s*Running "]
max_lines = 60
```

Target: push `make check` savings from 59.7% → 80%+. Measured via `rtk gain` before/after.

---

## 4. Fork RTK (Chantier parallel, separate repo)

- Fork `rtk-ai/rtk` → `zeyxx/rtk`
- Branch `cynic/export-api` for `rtk export --json --project <path> --since <duration>`
- PR upstream when stable (generic feature, not CYNIC-specific)
- CYNIC filters stay local (`.rtk/filters.toml` in CYNIC repo)

---

## 5. Dependencies

| Crate | Version | Feature | Why | Binary impact (est.) |
|-------|---------|---------|-----|---------------------|
| `rusqlite` | 0.34 | `bundled` | Read-only SQLite, zero system dep | ~1.5MB |

No new dependencies for filters (RTK reads `.rtk/filters.toml` natively).

---

## 6. What We Do NOT Build (Day 1)

| What | Why not | When |
|------|---------|------|
| `/metabolism` or `/organs` endpoint | No consumer identified (K15) | When Soma or dashboard needs it |
| Soma module | Emerges from observing organ patterns | After N≥3 organs or first contention incident |
| Dashboard/cynic-ui integration | No consumer | When we need visual metabolic awareness |
| Dynamic organ discovery | N=2, static suffices | When static registration becomes painful |
| Snapshot storage in SurrealDB | No consumer for historical snapshots | When rate computation needs history |
| OTel labels/attributes | No consumer filters by dimension | When first dashboard or query needs it |
| Metric naming registry | N=2, convention suffices | When namespace collision at N>4 |

---

## 7. Naming Convention (for N organ metrics)

```
<snake_case_name>
```

- Key: snake_case, descriptive (`tokens_saved`, `tweets_captured`, `dataset_bytes`)
- Unit: in `unit` field, never in key name (not `tokens_saved_count`)
- Kind: Counter for cumulative, Gauge for point-in-time
- Namespace: not needed at N=2. If collision at N>4: prefix with organ name (`rtk.tokens_saved`)

---

## 8. Falsification Table

| Claim | Test | Fail condition |
|-------|------|----------------|
| Filters push make check from 59.7% �� 80%+ | `rtk gain` before/after adding `.rtk/filters.toml` | savings_pct < 75% |
| rusqlite bundled doesn't bloat binary | `ls -la target/release/cynic-kernel` before/after | delta > 3MB |
| OrganPort carries N organs without kernel changes | Add Hermes X impl without touching `domain/organ.rs` | Any change needed in domain |
| Rate computation works with Counter+timestamp | Two snapshots 10min apart → Δ(tokens_saved)/Δt | Result is NaN or nonsensical |
| Read-only SQLite doesn't corrupt RTK data | `rtk gain` after 100+ kernel reads | Any data discrepancy |
| 1s busy_timeout handles contention | Snapshot during active `rtk` write | Panic or hang (should return Degraded) |

---

## 9. Emergence Watchlist (CHAOS→MATRIX)

Things we expect to learn from running this in production:

1. **Which metrics matter?** — We export 7 RTK metrics. Which ones does anyone actually look at? The unused ones are noise (BURN).
2. **Do we need historical snapshots?** — If yes → store in SurrealDB. If no → pull-only is fine.
3. **Does static registry hold?** — If adding organ 3 feels painful → dynamic discovery.
4. **Does the naming convention hold?** — First collision → namespace prefix.
5. **Does the trait need MetricKind extensions?** — First distribution analysis → add Histogram.
6. **What does Soma actually need?** — The pattern of cross-organ queries defines Soma's API.

These are not TODOs — they're observation targets. We watch for them and document when they emerge.

---

## 10. Spec Review — Issues & Resolutions

Spec reviewed by automated subagent. 6 issues found.

| # | Issue | Severity | Resolution |
|---|-------|----------|------------|
| 1 | **K15 violation**: `snapshot()` produces but no consumer acts on it day 1 | High | **Labelled as deliberate deferred-consumer.** Deadline: if no consumer in 30 days → delete the producer. The first consumer is expected to be either `/health` organ summary or Soma. Falsifiable. |
| 2 | **Blocking SQLite in async**: `open_db()` is sync inside `async fn`, will stall tokio threadpool | High | **Fixed.** All SQLite ops wrapped in `tokio::task::spawn_blocking()`. Added to `open_db()` doc. |
| 3 | **K14 violation**: `dirs::data_local_dir().unwrap_or_default()` silently drops organ if path empty | Medium | **Rejected.** K14 applies to shared state (RwLock, Option) where fallback affects system behavior. RTK is an optional organ — silent skip on absent `history.db` is correct. Registering a Dead organ for an uninstalled tool = noise. |
| 4 | **R1 violation**: `project_path LIKE '%CYNIC%'` hardcoded in SQL | Medium | **Fixed.** `RtkReader::new(db_path, project_root)` — project root passed at registry build time via `git rev-parse --show-toplevel` or equivalent. SQL uses bound parameter. |
| 5 | **C toolchain for rusqlite bundled**: not documented | Low | **Fixed.** `build-essential` verified present on cynic-core (observed). Documented in §11. |
| 6 | **Filters TOML native support**: inferred from README, not verified | Low | **Verified (observed).** RTK v0.30.1+ supports `rtk trust` / `rtk untrust` / `rtk verify` for project-local filters. Setup: `cd CYNIC && rtk trust` after creating `.rtk/filters.toml`. |

---

## 11. Prerequisites (before implementation)

### P0: Upgrade RTK to v0.37.2+

Local install is v0.30.1, upstream stable is v0.37.2 (7 minor versions behind). The SQLite schema documented in §1 was observed on v0.30.1 and **may have changed**.

```bash
# 1. Upgrade
cargo install rtk
# or: curl -fsSL https://rtk.sh | sh (if available)

# 2. Verify version
rtk --version  # expect >= 0.37.2

# 3. Re-verify SQLite schema (CRITICAL — the RtkReader depends on this)
sqlite3 ~/.local/share/rtk/history.db ".schema"

# 4. Compare with spec §1 schema — if columns added/renamed/removed, update RtkReader accordingly

# 5. Verify filters work
rtk gain  # should still show stats (backward compat)
```

**If schema changed:** update the SQL queries in RtkReader before implementation. The trait and types are schema-agnostic — only the adapter changes.

### P1: Fork RTK from upstream master

Fork from `rtk-ai/rtk` master (not from v0.30.1). The fork must track upstream to benefit from their active development (daily RC cadence).

```bash
gh repo fork rtk-ai/rtk --clone
cd rtk
git remote add upstream https://github.com/rtk-ai/rtk.git
```

### P2: Trust project-local filters

After creating `.rtk/filters.toml` in CYNIC repo:

```bash
cd /home/user/Bureau/CYNIC
rtk trust
rtk verify  # validate filter syntax
```

### Build requirements

- Rust 1.95.0 (current)
- `build-essential` (verified present on cynic-core — provides `cc` for rusqlite bundled)
- No additional system dependencies
