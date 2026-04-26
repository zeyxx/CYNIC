# OrganPort + RTK Integration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an `OrganPort` trait to the kernel domain layer and implement the first reader (`RtkReader`) that reads RTK's SQLite `history.db` in read-only mode, enabling metabolic awareness for the CYNIC organism.

**Architecture:** New `domain/organ.rs` defines the trait (async, Send+Sync, like all ports). New `senses/` module (distinct from existing `organ/` which is InferenceOrgan) holds concrete readers. `RtkReader` uses `rusqlite` (bundled) via `spawn_blocking`. Registry built at startup, injected into `AppState`.

**Tech Stack:** Rust 1.95.0, rusqlite 0.34 (bundled), async-trait, chrono, tokio::task::spawn_blocking.

**Spec:** `docs/superpowers/specs/2026-04-26-organ-port-rtk-integration-design.md`

**Prerequisites:**
- P0: Upgrade RTK to v0.37.2+ and re-verify SQLite schema (see spec §11)
- P2: `build-essential` present (verified)

---

## File Map

| Action | Path | Responsibility |
|--------|------|---------------|
| Create | `cynic-kernel/src/domain/organ.rs` | OrganPort trait, OrganHealth, Metric, MetricKind, MetricValue, OrganSnapshot, OrganError |
| Modify | `cynic-kernel/src/domain/mod.rs` | Add `pub mod organ;` |
| Create | `cynic-kernel/src/senses/mod.rs` | `pub mod rtk;` + `build_sense_registry()` |
| Create | `cynic-kernel/src/senses/rtk.rs` | RtkReader: impl OrganPort (rusqlite read-only) |
| Modify | `cynic-kernel/src/lib.rs` | Add `pub mod senses;` |
| Modify | `cynic-kernel/Cargo.toml` | Add `rusqlite = { version = "0.34", features = ["bundled"] }` |
| Modify | `cynic-kernel/src/api/rest/types.rs` | Add `pub senses: Vec<Arc<dyn OrganPort>>` to AppState |
| Modify | `cynic-kernel/src/main.rs` | Build sense registry, inject into AppState |
| Create | `cynic-kernel/tests/organ_port_contract.rs` | Contract tests for OrganPort trait |
| Create | `.rtk/filters.toml` | CYNIC project-local RTK filters |

**Why `senses/` not `organs/`?** The existing `organ/` module is the InferenceOrgan (Dog backend registry + health tracking). Our data-organ readers are sensory inputs — the organism perceiving external data stores. `senses/` avoids name collision and is semantically accurate.

---

## Task 1: OrganPort trait (domain layer)

**Files:**
- Create: `cynic-kernel/src/domain/organ.rs`
- Modify: `cynic-kernel/src/domain/mod.rs`
- Test: `cynic-kernel/tests/organ_port_contract.rs`

- [ ] **Step 1: Write the contract test file**

```rust
// tests/organ_port_contract.rs
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Contract tests for OrganPort — verifies the trait is object-safe,
//! can be boxed, and the types work as expected.

use cynic_kernel::domain::organ::{
    Metric, MetricKind, MetricValue, OrganError, OrganHealth, OrganPort, OrganSnapshot,
};
use async_trait::async_trait;
use chrono::Utc;
use std::time::Duration;

/// Stub organ for contract testing.
struct StubOrgan;

#[async_trait]
impl OrganPort for StubOrgan {
    fn name(&self) -> &str { "stub" }

    async fn health(&self) -> OrganHealth {
        OrganHealth::Alive
    }

    async fn freshness(&self) -> Result<Duration, OrganError> {
        Ok(Duration::from_secs(60))
    }

    async fn snapshot(&self) -> Result<OrganSnapshot, OrganError> {
        Ok(OrganSnapshot {
            taken_at: Utc::now(),
            metrics: vec![
                Metric {
                    key: "test_counter".to_string(),
                    value: MetricValue::I64(42),
                    kind: MetricKind::Counter,
                    unit: Some("count".to_string()),
                },
                Metric {
                    key: "test_gauge".to_string(),
                    value: MetricValue::F64(0.95),
                    kind: MetricKind::Gauge,
                    unit: Some("%".to_string()),
                },
            ],
        })
    }
}

#[tokio::test]
async fn trait_is_object_safe_and_boxable() {
    let organ: Box<dyn OrganPort> = Box::new(StubOrgan);
    assert_eq!(organ.name(), "stub");
    assert!(matches!(organ.health().await, OrganHealth::Alive));
}

#[tokio::test]
async fn snapshot_returns_typed_metrics() {
    let organ = StubOrgan;
    let snap = organ.snapshot().await.unwrap();
    assert_eq!(snap.metrics.len(), 2);
    assert!(matches!(snap.metrics[0].kind, MetricKind::Counter));
    assert!(matches!(snap.metrics[1].kind, MetricKind::Gauge));
    assert!(matches!(snap.metrics[0].value, MetricValue::I64(42)));
}

#[tokio::test]
async fn freshness_returns_duration() {
    let organ = StubOrgan;
    let fresh = organ.freshness().await.unwrap();
    assert_eq!(fresh.as_secs(), 60);
}

#[tokio::test]
async fn health_degraded_carries_reason() {
    struct DegradedOrgan;

    #[async_trait]
    impl OrganPort for DegradedOrgan {
        fn name(&self) -> &str { "degraded" }
        async fn health(&self) -> OrganHealth {
            OrganHealth::Degraded { reason: "db locked".to_string() }
        }
        async fn freshness(&self) -> Result<Duration, OrganError> {
            Err(OrganError::Unavailable("not reachable".to_string()))
        }
        async fn snapshot(&self) -> Result<OrganSnapshot, OrganError> {
            Err(OrganError::ReadFailed("timeout".to_string()))
        }
    }

    let organ = DegradedOrgan;
    match organ.health().await {
        OrganHealth::Degraded { reason } => assert_eq!(reason, "db locked"),
        _ => panic!("expected Degraded"),
    }
    assert!(organ.freshness().await.is_err());
    assert!(organ.snapshot().await.is_err());
}

#[tokio::test]
async fn organ_error_display() {
    let e = OrganError::Unavailable("gone".to_string());
    assert_eq!(format!("{e}"), "organ unavailable: gone");
    let e = OrganError::ReadFailed("io".to_string());
    assert_eq!(format!("{e}"), "organ read failed: io");
}
```

- [ ] **Step 2: Run test — verify it fails (module doesn't exist yet)**

```bash
cargo test --test organ_port_contract 2>&1 | tail -5
```

Expected: compilation error — `cynic_kernel::domain::organ` not found.

- [ ] **Step 3: Create domain/organ.rs with the trait and types**

```rust
// domain/organ.rs
//! OrganPort — domain contract for organism sensory organs.
//!
//! Each organ owns its data store (SQLite, files, etc). The kernel reads
//! organ data in read-only mode via this trait. Designed for N organs
//! with N functions and N states — zero kernel changes per new organ.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::time::Duration;

/// Domain contract for an organism sensory organ.
///
/// All methods are async to align with existing port patterns (StoragePort,
/// BackendPort). Concrete impls that do blocking I/O (e.g. rusqlite) must
/// use `tokio::task::spawn_blocking()` internally.
#[async_trait]
pub trait OrganPort: Send + Sync {
    /// Stable organ identity (e.g. "rtk", "hermes-x").
    fn name(&self) -> &str;

    /// Organ liveness. Degraded/Dead carry a reason string.
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
    /// Data source not found (DB missing, file absent).
    Unavailable(String),
    /// Data source found but read failed (IO, parse, lock timeout).
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

- [ ] **Step 4: Add `pub mod organ;` to domain/mod.rs**

Add after existing modules (line ~26 of `cynic-kernel/src/domain/mod.rs`):
```rust
pub mod organ;
```

- [ ] **Step 5: Run tests — verify they pass**

```bash
cargo test --test organ_port_contract 2>&1 | tail -10
```

Expected: all 5 tests pass.

- [ ] **Step 6: Run clippy**

```bash
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -5
```

Expected: 0 warnings.

- [ ] **Step 7: Commit**

```bash
git add cynic-kernel/src/domain/organ.rs cynic-kernel/src/domain/mod.rs cynic-kernel/tests/organ_port_contract.rs
git commit -m "feat(domain): add OrganPort trait for N-organ sensory input"
```

---

## Task 2: RtkReader implementation

**Files:**
- Modify: `cynic-kernel/Cargo.toml` (add rusqlite)
- Create: `cynic-kernel/src/senses/mod.rs`
- Create: `cynic-kernel/src/senses/rtk.rs`
- Modify: `cynic-kernel/src/lib.rs` (add pub mod senses)
- Test: inline in `senses/rtk.rs` (#[cfg(test)] module)

- [ ] **Step 1: Add rusqlite dependency**

In `cynic-kernel/Cargo.toml`, in `[dependencies]` section:
```toml
rusqlite = { version = "0.34", features = ["bundled"] }
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build -p cynic-kernel 2>&1 | tail -5
```

Expected: compiles (may take longer first time due to SQLite C compilation). Note binary size for falsification (§8 of spec).

```bash
ls -la target/release/cynic-kernel 2>/dev/null || ls -la target/debug/cynic-kernel
```

- [ ] **Step 3: Create senses/rtk.rs with RtkReader**

```rust
// senses/rtk.rs
//! RtkReader — reads RTK's history.db (SQLite) in read-only mode.
//! All SQLite operations run inside spawn_blocking to avoid stalling tokio.

use crate::domain::organ::{
    Metric, MetricKind, MetricValue, OrganError, OrganHealth, OrganPort, OrganSnapshot,
};
use async_trait::async_trait;
use chrono::Utc;
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;
use std::time::Duration;

pub struct RtkReader {
    db_path: PathBuf,
    project_root: String, // used in SQL LIKE filter — no hardcoded paths (R1)
}

impl RtkReader {
    pub fn new(db_path: PathBuf, project_root: String) -> Self {
        Self {
            db_path,
            project_root,
        }
    }

    /// Open DB read-only. Called inside spawn_blocking only.
    fn open_db(path: &PathBuf) -> Result<Connection, OrganError> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| OrganError::ReadFailed(format!("sqlite open: {e}")))?;

        conn.busy_timeout(Duration::from_secs(1))
            .map_err(|e| OrganError::ReadFailed(format!("busy_timeout: {e}")))?;

        Ok(conn)
    }
}

#[async_trait]
impl OrganPort for RtkReader {
    fn name(&self) -> &str {
        "rtk"
    }

    async fn health(&self) -> OrganHealth {
        let path = self.db_path.clone();
        let result = tokio::task::spawn_blocking(move || {
            if !path.exists() {
                return OrganHealth::Dead {
                    reason: "history.db not found".to_string(),
                };
            }
            match Self::open_db(&path) {
                Ok(_) => OrganHealth::Alive,
                Err(OrganError::ReadFailed(reason)) => OrganHealth::Degraded { reason },
                Err(OrganError::Unavailable(reason)) => OrganHealth::Dead { reason },
            }
        })
        .await;

        // K14: spawn failure = assume degraded
        result.unwrap_or(OrganHealth::Degraded {
            reason: "spawn_blocking panicked".to_string(),
        })
    }

    async fn freshness(&self) -> Result<Duration, OrganError> {
        let path = self.db_path.clone();
        let result = tokio::task::spawn_blocking(move || {
            let conn = Self::open_db(&path)?;
            let ts: String = conn
                .query_row(
                    "SELECT MAX(timestamp) FROM commands",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| OrganError::ReadFailed(format!("freshness query: {e}")))?;

            // RTK timestamps are ISO 8601
            let parsed = chrono::DateTime::parse_from_rfc3339(&ts)
                .map_err(|e| OrganError::ReadFailed(format!("timestamp parse: {e}")))?;

            let age = Utc::now()
                .signed_duration_since(parsed)
                .to_std()
                .unwrap_or(Duration::ZERO);

            Ok(age)
        })
        .await
        .map_err(|e| OrganError::ReadFailed(format!("spawn_blocking: {e}")))?
    }

    async fn snapshot(&self) -> Result<OrganSnapshot, OrganError> {
        let path = self.db_path.clone();
        let project_filter = format!("%{}%", self.project_root);

        let result = tokio::task::spawn_blocking(move || {
            let conn = Self::open_db(&path)?;

            let (cmd_count, input_tok, output_tok, saved_tok, avg_pct, exec_ms): (
                i64, i64, i64, i64, f64, i64,
            ) = conn
                .query_row(
                    "SELECT COUNT(*), \
                            COALESCE(SUM(input_tokens), 0), \
                            COALESCE(SUM(output_tokens), 0), \
                            COALESCE(SUM(saved_tokens), 0), \
                            COALESCE(AVG(savings_pct), 0.0), \
                            COALESCE(SUM(exec_time_ms), 0) \
                     FROM commands WHERE project_path LIKE ?1",
                    [&project_filter],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
                )
                .map_err(|e| OrganError::ReadFailed(format!("snapshot query: {e}")))?;

            let parse_failures: i64 = conn
                .query_row("SELECT COUNT(*) FROM parse_failures", [], |row| row.get(0))
                .map_err(|e| OrganError::ReadFailed(format!("parse_failures query: {e}")))?;

            Ok(OrganSnapshot {
                taken_at: Utc::now(),
                metrics: vec![
                    Metric { key: "commands_total".into(), value: MetricValue::I64(cmd_count), kind: MetricKind::Counter, unit: Some("count".into()) },
                    Metric { key: "tokens_input".into(), value: MetricValue::I64(input_tok), kind: MetricKind::Counter, unit: Some("tokens".into()) },
                    Metric { key: "tokens_output".into(), value: MetricValue::I64(output_tok), kind: MetricKind::Counter, unit: Some("tokens".into()) },
                    Metric { key: "tokens_saved".into(), value: MetricValue::I64(saved_tok), kind: MetricKind::Counter, unit: Some("tokens".into()) },
                    Metric { key: "savings_pct".into(), value: MetricValue::F64(avg_pct), kind: MetricKind::Gauge, unit: Some("%".into()) },
                    Metric { key: "exec_time_total".into(), value: MetricValue::I64(exec_ms), kind: MetricKind::Counter, unit: Some("ms".into()) },
                    Metric { key: "parse_failures".into(), value: MetricValue::I64(parse_failures), kind: MetricKind::Counter, unit: Some("count".into()) },
                ],
            })
        })
        .await
        .map_err(|e| OrganError::ReadFailed(format!("spawn_blocking: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn health_dead_when_db_missing() {
        let reader = RtkReader::new(PathBuf::from("/tmp/nonexistent.db"), "/tmp".to_string());
        assert!(matches!(reader.health().await, OrganHealth::Dead { .. }));
    }

    #[tokio::test]
    async fn freshness_fails_when_db_missing() {
        let reader = RtkReader::new(PathBuf::from("/tmp/nonexistent.db"), "/tmp".to_string());
        assert!(reader.freshness().await.is_err());
    }

    #[tokio::test]
    async fn snapshot_fails_when_db_missing() {
        let reader = RtkReader::new(PathBuf::from("/tmp/nonexistent.db"), "/tmp".to_string());
        assert!(reader.snapshot().await.is_err());
    }
}
```

- [ ] **Step 4: Create senses/mod.rs with registry builder**

```rust
// senses/mod.rs
//! Sensory organ readers — the organism perceiving external data stores.
//!
//! Distinct from organ/ (InferenceOrgan: Dog backend registry).
//! Each sense reads an external data store in read-only mode.

pub mod rtk;

use crate::domain::organ::OrganPort;
use std::sync::Arc;

/// Build the sense registry at startup. Best-effort: missing organs are skipped.
/// Returns empty vec if no organs are available — this is fine (non-fatal).
pub fn build_sense_registry(project_root: &str) -> Vec<Arc<dyn OrganPort>> {
    let mut senses: Vec<Arc<dyn OrganPort>> = Vec::new();

    // RTK — optional, skip if history.db not found
    let rtk_db = dirs::data_local_dir()
        .unwrap_or_default()
        .join("rtk/history.db");
    if rtk_db.exists() {
        senses.push(Arc::new(rtk::RtkReader::new(
            rtk_db,
            project_root.to_string(),
        )));
    }

    senses
}
```

- [ ] **Step 5: Add `pub mod senses;` to lib.rs**

After existing module declarations in `cynic-kernel/src/lib.rs`:
```rust
pub mod senses;
```

- [ ] **Step 6: Run build + tests**

```bash
cargo build -p cynic-kernel 2>&1 | tail -5
cargo test -p cynic-kernel --lib -- senses::rtk 2>&1 | tail -10
```

Expected: build passes, 3 rtk tests pass.

- [ ] **Step 7: Run clippy**

```bash
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -5
```

- [ ] **Step 8: Commit**

```bash
git add cynic-kernel/Cargo.toml cynic-kernel/src/senses/ cynic-kernel/src/lib.rs
git commit -m "feat(senses): add RtkReader — first OrganPort impl (read-only SQLite)"
```

---

## Task 3: Wire into AppState + verify end-to-end

**Files:**
- Modify: `cynic-kernel/src/api/rest/types.rs` (~line 26)
- Modify: `cynic-kernel/src/main.rs` (startup assembly)

- [ ] **Step 1: Add senses field to AppState**

In `cynic-kernel/src/api/rest/types.rs`, add import:
```rust
use crate::domain::organ::OrganPort;
```

And add field to `AppState` struct (after `judge_jobs`):
```rust
    /// Sensory organ readers — organism perceiving external data stores.
    /// K15 deferred-consumer: deadline 30 days (2026-05-26). If no consumer by then, remove.
    pub senses: Vec<Arc<dyn OrganPort>>,
```

- [ ] **Step 2: Build sense registry in main.rs startup**

Find where `AppState` is constructed in `main.rs`. Add before the `AppState` construction:
```rust
    let project_root = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let senses = cynic_kernel::senses::build_sense_registry(&project_root);
    if !senses.is_empty() {
        klog!("[senses] {} organ(s) registered", senses.len());
        for s in &senses {
            klog!("[senses]   → {}", s.name());
        }
    }
```

And add to the `AppState { ... }` struct literal:
```rust
        senses,
```

- [ ] **Step 3: Build and verify**

```bash
cargo build -p cynic-kernel 2>&1 | tail -5
```

Expected: compiles.

- [ ] **Step 4: Run full test suite**

```bash
cargo test -p cynic-kernel 2>&1 | tail -15
```

Expected: all existing tests + new tests pass. No regressions.

- [ ] **Step 5: Run clippy**

```bash
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -5
```

- [ ] **Step 6: Verify binary size delta**

```bash
cargo build --release -p cynic-kernel 2>&1 | tail -3
ls -la target/release/cynic-kernel
```

Compare with pre-rusqlite binary size. Delta should be < 3MB (spec §8 falsification).

- [ ] **Step 7: Commit**

```bash
git add cynic-kernel/src/api/rest/types.rs cynic-kernel/src/main.rs
git commit -m "feat(senses): wire organ registry into AppState at startup"
```

---

## Task 4: Integration test with real history.db

**Files:**
- Create: `cynic-kernel/tests/integration_rtk_reader.rs`

- [ ] **Step 1: Write integration test**

```rust
// tests/integration_rtk_reader.rs
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration test for RtkReader against real history.db.
//! #[ignore] — requires RTK installed with data.

use cynic_kernel::domain::organ::{MetricKind, MetricValue, OrganHealth, OrganPort};
use cynic_kernel::senses::rtk::RtkReader;
use std::path::PathBuf;

fn rtk_db_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join("rtk/history.db")
}

#[tokio::test]
#[ignore] // Requires real RTK history.db
async fn rtk_health_alive_with_real_db() {
    let db = rtk_db_path();
    if !db.exists() {
        eprintln!("SKIP: RTK history.db not found at {}", db.display());
        return;
    }
    let reader = RtkReader::new(db, "/home/user/Bureau/CYNIC".to_string());
    assert!(matches!(reader.health().await, OrganHealth::Alive));
}

#[tokio::test]
#[ignore]
async fn rtk_freshness_returns_reasonable_duration() {
    let db = rtk_db_path();
    if !db.exists() { return; }
    let reader = RtkReader::new(db, "/home/user/Bureau/CYNIC".to_string());
    let fresh = reader.freshness().await.unwrap();
    // Should be < 7 days if RTK is actively used
    assert!(fresh.as_secs() < 7 * 24 * 3600, "freshness too old: {:?}", fresh);
}

#[tokio::test]
#[ignore]
async fn rtk_snapshot_returns_7_metrics() {
    let db = rtk_db_path();
    if !db.exists() { return; }
    let reader = RtkReader::new(db, "/home/user/Bureau/CYNIC".to_string());
    let snap = reader.snapshot().await.unwrap();

    assert_eq!(snap.metrics.len(), 7);

    // Verify expected keys exist
    let keys: Vec<&str> = snap.metrics.iter().map(|m| m.key.as_str()).collect();
    assert!(keys.contains(&"commands_total"));
    assert!(keys.contains(&"tokens_saved"));
    assert!(keys.contains(&"savings_pct"));
    assert!(keys.contains(&"parse_failures"));

    // tokens_saved should be > 0 (we know there's 62M+ saved)
    let saved = snap.metrics.iter().find(|m| m.key == "tokens_saved").unwrap();
    match saved.value {
        MetricValue::I64(v) => assert!(v > 0, "tokens_saved should be positive: {v}"),
        _ => panic!("tokens_saved should be I64"),
    }
    assert!(matches!(saved.kind, MetricKind::Counter));

    // savings_pct should be a gauge between 0 and 100
    let pct = snap.metrics.iter().find(|m| m.key == "savings_pct").unwrap();
    match pct.value {
        MetricValue::F64(v) => assert!((0.0..=100.0).contains(&v), "savings_pct out of range: {v}"),
        _ => panic!("savings_pct should be F64"),
    }
    assert!(matches!(pct.kind, MetricKind::Gauge));
}

#[tokio::test]
#[ignore]
async fn rtk_readonly_preserves_data_across_reads() {
    let db = rtk_db_path();
    if !db.exists() { return; }
    let reader = RtkReader::new(db, "/home/user/Bureau/CYNIC".to_string());

    // Snapshot before
    let before = reader.snapshot().await.unwrap();
    let count_before = before.metrics.iter()
        .find(|m| m.key == "commands_total")
        .map(|m| match m.value { MetricValue::I64(v) => v, _ => -1 })
        .unwrap();

    // Take 10 snapshots rapidly (simulates concurrent reads)
    for _ in 0..10 {
        let _ = reader.snapshot().await;
    }

    // Snapshot after — commands_total must not have changed (read-only)
    let after = reader.snapshot().await.unwrap();
    let count_after = after.metrics.iter()
        .find(|m| m.key == "commands_total")
        .map(|m| match m.value { MetricValue::I64(v) => v, _ => -1 })
        .unwrap();

    assert_eq!(count_before, count_after, "read-only access should not change data");
    assert_eq!(after.metrics.len(), 7);
}
```

- [ ] **Step 2: Run integration test**

```bash
cargo test --test integration_rtk_reader -- --ignored 2>&1 | tail -15
```

Expected: all 4 tests pass (RTK history.db exists on this machine).

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/tests/integration_rtk_reader.rs
git commit -m "test(senses): integration tests for RtkReader against real history.db"
```

---

## Task 5: CYNIC project-local RTK filters

**Files:**
- Create: `.rtk/filters.toml`

- [ ] **Step 1: Record baseline savings**

```bash
rtk gain 2>&1 | head -20
```

Note the `make check` and `cargo test` savings_pct.

- [ ] **Step 2: Create .rtk/ directory and filters.toml**

```bash
mkdir -p .rtk
```

```toml
# CYNIC project-local RTK filters — trusted via `rtk trust`.
# Targets: push make check from 59.7% → 80%+, cargo test from 87% → 95%+.
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

- [ ] **Step 3: Trust the filters**

```bash
cd /home/user/Bureau/CYNIC && rtk trust
rtk verify
```

Expected: filters trusted, verify passes.

- [ ] **Step 4: Test filters work**

```bash
make check 2>&1 | wc -l   # should be shorter than before
rtk gain 2>&1 | head -20   # check if savings improved
```

- [ ] **Step 5: Commit**

```bash
git add .rtk/filters.toml
git commit -m "ops(rtk): add CYNIC project-local filters for cargo/make output"
```

---

## Task 6: Full gate validation

- [ ] **Step 1: Run make check (full gate — includes lint-rules + lint-drift)**

```bash
export RUST_MIN_STACK=67108864
export RUSTFLAGS="-C debuginfo=1"
make check
```

This runs build + test + clippy + lint-rules + lint-drift (K15/K17 audit). All must pass.

- [ ] **Step 2: Run all tests including integration**

```bash
cargo test -p cynic-kernel 2>&1 | tail -15
cargo test --test integration_rtk_reader -- --ignored 2>&1 | tail -10
cargo test --test organ_port_contract 2>&1 | tail -10
```

- [ ] **Step 3: Verify falsification claims from spec §8**

```bash
# Binary size delta
ls -la target/release/cynic-kernel

# RTK data integrity
rtk gain | head -5

# Filter improvement
# Compare with baseline from Task 5 Step 1
```

- [ ] **Step 4: Final commit (if any remaining changes)**

```bash
git status
# If clean: done. If dirty: stage and commit.
```
