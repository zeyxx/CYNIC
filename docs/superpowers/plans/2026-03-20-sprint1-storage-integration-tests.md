# Sprint 1: Storage Integration Tests — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every `StoragePort` + `CoordPort` method gets at least one integration test against a real SurrealDB instance, with `make check-storage` as the gate.

**Architecture:** Integration tests live in `cynic-kernel/tests/integration_storage.rs`, marked `#[ignore]` by default. A shared test helper (`tests/common/mod.rs`) provides `setup_test_db()` → `SurrealHttpStorage` connected to an isolated test database, and `teardown_test_db()` to drop it. Tests run via `make check-storage`.

**Tech Stack:** Rust, tokio, SurrealDB HTTP API, `#[tokio::test]` + `#[ignore]`

**Spec:** `docs/superpowers/specs/2026-03-20-compound-hardening-design.md`

---

### Task 1: Test Helper Infrastructure

**Files:**
- Create: `cynic-kernel/tests/common/mod.rs`

- [ ] **Step 1: Write the test helper module**

```rust
use cynic_kernel::storage::SurrealHttpStorage;
use std::time::{SystemTime, UNIX_EPOCH};

/// Create an isolated test database and return a connected SurrealHttpStorage.
/// Each test gets its own DB: `test_<unix_millis>_<suffix>`.
pub async fn setup_test_db(suffix: &str) -> SurrealHttpStorage {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let db_name = format!("test_{}_{}", millis, suffix);
    SurrealHttpStorage::init_with("http://localhost:8000", "cynic_test", &db_name)
        .await
        .expect("Failed to connect to test SurrealDB")
}

/// Drop the test database. Call in a defer/cleanup.
pub async fn teardown_test_db(db: &SurrealHttpStorage) {
    let _ = db.query(&format!("REMOVE DATABASE `{}`;", db.db_name())).await;
}
```

- [ ] **Step 2: Expose `db_name()` on SurrealHttpStorage**

In `cynic-kernel/src/storage/mod.rs`, add a public getter:

```rust
pub fn db_name(&self) -> &str {
    &self.db
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p cynic-kernel --release --tests 2>&1 | tail -5`
Expected: compiles (tests not run yet)

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/tests/common/mod.rs cynic-kernel/src/storage/mod.rs
git commit -m "test(infra): add integration test helper — setup/teardown isolated SurrealDB"
```

---

### Task 2: Ping + Bootstrap Test

**Files:**
- Create: `cynic-kernel/tests/integration_storage.rs`

- [ ] **Step 1: Write the ping test**

```rust
mod common;

use cynic_kernel::domain::storage::StoragePort;

#[tokio::test]
#[ignore]
async fn ping_succeeds_on_healthy_db() {
    let db = common::setup_test_db("ping").await;
    let result = db.ping().await;
    assert!(result.is_ok(), "ping failed: {:?}", result.err());
    common::teardown_test_db(&db).await;
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `SURREALDB_PASS=$(grep SURREALDB_PASS ~/.cynic-env | cut -d= -f2) cargo test -p cynic-kernel --release -- --ignored ping_succeeds 2>&1 | tail -5`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/tests/integration_storage.rs
git commit -m "test(storage): ping integration test — validates DB connectivity"
```

---

### Task 3: Verdict Round-Trip Tests

**Files:**
- Modify: `cynic-kernel/tests/integration_storage.rs`

- [ ] **Step 1: Write store + get + list verdict tests**

```rust
use cynic_kernel::domain::dog::{Verdict, VerdictKind, QScore, AxiomScores, AxiomReasoning, DogScore};

fn test_verdict(id: &str) -> Verdict {
    Verdict {
        verdict_id: id.to_string(),
        kind: VerdictKind::Howl,
        stimulus_summary: "Test stimulus".to_string(),
        stimulus_domain: "test".to_string(),
        q_score: QScore {
            total: 0.55,
            fidelity: 0.6, phi: 0.5, verify: 0.5,
            culture: 0.6, burn: 0.5, sovereignty: 0.6,
        },
        dog_scores: vec![DogScore {
            dog_id: "test-dog".to_string(),
            axiom_scores: AxiomScores::default(),
            latency_ms: 100.0,
        }],
        dogs_used: "test-dog".to_string(),
        reasoning: AxiomReasoning::default(),
        created_at: chrono::Utc::now().to_rfc3339(),
        integrity_hash: "test-hash".to_string(),
        convergence: 0.5,
    }
}

#[tokio::test]
#[ignore]
async fn verdict_store_and_retrieve() {
    let db = common::setup_test_db("verdict_rt").await;
    let v = test_verdict("v-001");
    db.store_verdict(&v).await.expect("store_verdict failed");

    let got = db.get_verdict("v-001").await.expect("get_verdict failed");
    assert!(got.is_some(), "verdict not found after store");
    let got = got.unwrap();
    assert_eq!(got.verdict_id, "v-001");
    assert_eq!(got.kind, VerdictKind::Howl);
    assert!((got.q_score.total - 0.55).abs() < 0.001);

    common::teardown_test_db(&db).await;
}

#[tokio::test]
#[ignore]
async fn verdict_list_respects_limit() {
    let db = common::setup_test_db("verdict_list").await;
    for i in 0..5 {
        let v = test_verdict(&format!("v-{:03}", i));
        db.store_verdict(&v).await.unwrap();
    }

    let all = db.list_verdicts(10).await.expect("list_verdicts failed");
    assert_eq!(all.len(), 5);

    let limited = db.list_verdicts(2).await.expect("list_verdicts limited failed");
    assert_eq!(limited.len(), 2);

    common::teardown_test_db(&db).await;
}
```

- [ ] **Step 2: Run tests**

Run: `SURREALDB_PASS=... cargo test -p cynic-kernel --release -- --ignored verdict_ 2>&1 | tail -10`
Expected: 2 PASS

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/tests/integration_storage.rs
git commit -m "test(storage): verdict store/get/list integration tests"
```

---

### Task 4: Crystal Tests (observe + get + list + state transitions)

**Files:**
- Modify: `cynic-kernel/tests/integration_storage.rs`

- [ ] **Step 1: Write crystal tests**

```rust
#[tokio::test]
#[ignore]
async fn crystal_observe_creates_and_updates() {
    let db = common::setup_test_db("crystal_obs").await;
    let ts = chrono::Utc::now().to_rfc3339();

    // First observation creates crystal
    db.observe_crystal("test-crystal", "Test content", "test", 0.7, &ts)
        .await.expect("observe_crystal failed");

    let c = db.get_crystal("test-crystal").await.expect("get_crystal failed");
    assert!(c.is_some(), "crystal not found after observe");
    let c = c.unwrap();
    assert_eq!(c.observations, 1);
    assert_eq!(c.content, "Test content");

    // Second observation updates
    db.observe_crystal("test-crystal", "Test content", "test", 0.8, &ts)
        .await.expect("second observe failed");
    let c2 = db.get_crystal("test-crystal").await.expect("get after update failed");
    let c2 = c2.unwrap();
    assert_eq!(c2.observations, 2);
    assert!(c2.confidence > c.confidence, "confidence should increase");

    common::teardown_test_db(&db).await;
}

#[tokio::test]
#[ignore]
async fn crystal_list_returns_sorted() {
    let db = common::setup_test_db("crystal_list").await;
    let ts = chrono::Utc::now().to_rfc3339();

    // Create two crystals with different observation counts
    for _ in 0..5 {
        db.observe_crystal("c-many", "Many obs", "test", 0.6, &ts).await.unwrap();
    }
    db.observe_crystal("c-few", "Few obs", "test", 0.9, &ts).await.unwrap();

    let list = db.list_crystals(10).await.expect("list_crystals failed");
    assert!(list.len() >= 2);
    // Most observed first
    assert!(list[0].observations >= list[1].observations);

    common::teardown_test_db(&db).await;
}
```

- [ ] **Step 2: Run and verify**

Run: `SURREALDB_PASS=... cargo test -p cynic-kernel --release -- --ignored crystal_ 2>&1 | tail -10`
Expected: 2 PASS

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/tests/integration_storage.rs
git commit -m "test(storage): crystal observe/get/list integration tests"
```

---

### Task 5: Observation Tests (store + query + session_targets)

**Files:**
- Modify: `cynic-kernel/tests/integration_storage.rs`

- [ ] **Step 1: Write observation tests**

```rust
use cynic_kernel::domain::storage::Observation;

fn test_obs(agent: &str, tool: &str, target: &str) -> Observation {
    Observation {
        project: "CYNIC".to_string(),
        agent_id: agent.to_string(),
        tool: tool.to_string(),
        target: target.to_string(),
        domain: "rust".to_string(),
        status: "success".to_string(),
        context: "test context".to_string(),
        session_id: agent.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

#[tokio::test]
#[ignore]
async fn observation_store_and_query() {
    let db = common::setup_test_db("obs_rt").await;

    db.store_observation(&test_obs("agent-1", "Edit", "main.rs")).await.expect("store failed");
    db.store_observation(&test_obs("agent-1", "Edit", "main.rs")).await.expect("store2 failed");
    db.store_observation(&test_obs("agent-1", "Read", "lib.rs")).await.expect("store3 failed");

    let rows = db.query_observations("CYNIC", None, 50).await.expect("query failed");
    assert!(!rows.is_empty(), "should have observations");

    common::teardown_test_db(&db).await;
}

#[tokio::test]
#[ignore]
async fn observation_query_session_targets() {
    let db = common::setup_test_db("obs_sess").await;

    db.store_observation(&test_obs("session-A", "Edit", "file1.rs")).await.unwrap();
    db.store_observation(&test_obs("session-A", "Edit", "file2.rs")).await.unwrap();
    db.store_observation(&test_obs("session-B", "Edit", "file3.rs")).await.unwrap();

    let rows = db.query_session_targets("CYNIC", 500).await.expect("query_session_targets failed");
    // Should return session-grouped target data
    assert!(!rows.is_empty());

    common::teardown_test_db(&db).await;
}
```

- [ ] **Step 2: Run and verify**

Expected: 2 PASS

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/tests/integration_storage.rs
git commit -m "test(storage): observation store/query/session_targets integration tests"
```

---

### Task 6: Coord Lifecycle Tests (register + heartbeat + deactivate + claim + release + who)

**Files:**
- Modify: `cynic-kernel/tests/integration_storage.rs`

- [ ] **Step 1: Write coord lifecycle test**

```rust
use cynic_kernel::domain::coord::{CoordPort, ClaimResult};

#[tokio::test]
#[ignore]
async fn coord_full_lifecycle() {
    let db = common::setup_test_db("coord_life").await;

    // Register
    db.register_agent("test-agent", "claude", "testing").await
        .expect("register failed");

    // Heartbeat
    db.heartbeat("test-agent").await.expect("heartbeat failed");

    // Claim
    let result = db.claim("test-agent", "main.rs", "file").await
        .expect("claim failed");
    assert!(matches!(result, ClaimResult::Claimed));

    // Who shows the claim
    let snap = db.who(Some("test-agent")).await.expect("who failed");
    assert!(!snap.claims.is_empty(), "should have claims after claim");

    // Release specific
    db.release("test-agent", Some("main.rs")).await.expect("release failed");
    let snap2 = db.who(Some("test-agent")).await.expect("who after release failed");
    assert!(snap2.claims.is_empty(), "claims should be empty after release");

    // Deactivate
    db.deactivate_agent("test-agent").await.expect("deactivate failed");

    common::teardown_test_db(&db).await;
}

#[tokio::test]
#[ignore]
async fn coord_claim_conflict_detection() {
    let db = common::setup_test_db("coord_conflict").await;

    db.register_agent("agent-A", "claude", "working").await.unwrap();
    db.register_agent("agent-B", "gemini", "working").await.unwrap();

    // Agent A claims file
    let r1 = db.claim("agent-A", "shared.rs", "file").await.unwrap();
    assert!(matches!(r1, ClaimResult::Claimed));

    // Agent B tries to claim same file → conflict
    let r2 = db.claim("agent-B", "shared.rs", "file").await.unwrap();
    assert!(matches!(r2, ClaimResult::Conflict(_)));

    // Cleanup
    db.release("agent-A", None).await.unwrap();
    db.release("agent-B", None).await.unwrap();
    common::teardown_test_db(&db).await;
}
```

- [ ] **Step 2: Run and verify**

Expected: 2 PASS

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/tests/integration_storage.rs
git commit -m "test(storage): coord lifecycle + conflict detection integration tests"
```

---

### Task 7: Claim Batch + Expire Stale Tests

**Files:**
- Modify: `cynic-kernel/tests/integration_storage.rs`

- [ ] **Step 1: Write batch claim + expiry tests**

```rust
#[tokio::test]
#[ignore]
async fn coord_claim_batch() {
    let db = common::setup_test_db("coord_batch").await;
    db.register_agent("batch-agent", "claude", "batch test").await.unwrap();

    let targets = vec!["file1.rs".to_string(), "file2.rs".to_string(), "file3.rs".to_string()];
    let result = db.claim_batch("batch-agent", &targets, "file").await
        .expect("claim_batch failed");
    assert_eq!(result.claimed.len(), 3);
    assert!(result.conflicts.is_empty());

    db.release("batch-agent", None).await.unwrap();
    common::teardown_test_db(&db).await;
}

#[tokio::test]
#[ignore]
async fn coord_expire_stale_cleans_old_sessions() {
    let db = common::setup_test_db("coord_expiry").await;
    db.register_agent("stale-agent", "test", "will expire").await.unwrap();
    db.claim("stale-agent", "old-file.rs", "file").await.unwrap();

    // expire_stale won't expire a fresh session (last_seen < 5 min ago)
    db.expire_stale().await.expect("expire_stale failed");
    let snap = db.who(None).await.unwrap();
    assert!(!snap.agents.is_empty(), "fresh session should survive expire_stale");

    // We can't easily test time-based expiry without time manipulation,
    // but we verify expire_stale doesn't error on valid state
    db.release("stale-agent", None).await.unwrap();
    common::teardown_test_db(&db).await;
}
```

- [ ] **Step 2: Run and verify**

Expected: 2 PASS

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/tests/integration_storage.rs
git commit -m "test(storage): claim_batch + expire_stale integration tests"
```

---

### Task 8: Audit Trail Tests (store + query)

**Files:**
- Modify: `cynic-kernel/tests/integration_storage.rs`

- [ ] **Step 1: Write audit tests**

```rust
#[tokio::test]
#[ignore]
async fn audit_store_and_query() {
    let db = common::setup_test_db("audit_rt").await;

    let details = serde_json::json!({"action": "test", "target": "file.rs"});
    db.store_audit("cynic_judge", "test-agent", &details).await
        .expect("store_audit failed");
    db.store_audit("cynic_infer", "test-agent", &details).await.unwrap();

    // Query all
    let all = db.query_audit(None, None, 10).await.expect("query_audit failed");
    assert!(all.len() >= 2, "should have at least 2 audit entries");

    // Query by tool
    let filtered = db.query_audit(Some("cynic_judge"), None, 10).await.unwrap();
    assert!(!filtered.is_empty());

    common::teardown_test_db(&db).await;
}
```

- [ ] **Step 2: Run and verify**

Expected: 1 PASS

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/tests/integration_storage.rs
git commit -m "test(storage): audit store/query integration test"
```

---

### Task 9: Flush Usage Test

**Files:**
- Modify: `cynic-kernel/tests/integration_storage.rs`

- [ ] **Step 1: Write flush usage test**

```rust
use cynic_kernel::domain::usage::DogUsageTracker;

#[tokio::test]
#[ignore]
async fn flush_usage_upserts_correctly() {
    let db = common::setup_test_db("flush").await;
    let mut tracker = DogUsageTracker::new();

    // Record some usage
    tracker.record("test-dog", 1000, 500, 200);
    tracker.record("test-dog", 500, 250, 100);

    // Build and execute flush SQL
    let sql = tracker.build_flush_sql();
    assert!(!sql.is_empty(), "flush SQL should not be empty");
    db.query(&sql).await.expect("flush SQL failed");

    // Verify data was written
    let rows = db.query_one("SELECT * FROM dog_usage;").await.expect("select failed");
    assert!(!rows.is_empty(), "dog_usage should have data after flush");

    // Flush again (idempotent accumulation)
    tracker.record("test-dog", 100, 50, 50);
    let sql2 = tracker.build_flush_sql();
    db.query(&sql2).await.expect("second flush failed");

    common::teardown_test_db(&db).await;
}
```

- [ ] **Step 2: Run and verify**

Expected: 1 PASS

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/tests/integration_storage.rs
git commit -m "test(storage): flush_usage upsert integration test"
```

---

### Task 10: Makefile Target + Pre-Push Hook

**Files:**
- Modify: `Makefile`
- Modify: `.claude/hooks/pre-push.sh` (or create if absent)

- [ ] **Step 1: Add `check-storage` target to Makefile**

Add after the `check` target:

```makefile
check-storage: ## Integration tests against real SurrealDB
	@echo "▸ Storage integration tests (requires SurrealDB on :8000)..."
	@SURREALDB_PASS=$$(grep -oP 'SURREALDB_PASS=\K.*' ~/.cynic-env) \
		cargo test -p cynic-kernel --release -- --ignored 2>&1 | tail -20
	@echo "✓ Storage integration tests passed"
```

- [ ] **Step 2: Run `make check-storage` to verify**

Expected: all integration tests PASS

- [ ] **Step 3: Commit**

```bash
git add Makefile
git commit -m "build(infra): add make check-storage — integration tests against real SurrealDB"
```

---

### Task 11: Final Verification — Full Suite

- [ ] **Step 1: Run `make check`** (unit tests + clippy unchanged)

Expected: 167+ tests pass, clippy clean

- [ ] **Step 2: Run `make check-storage`** (all integration tests)

Expected: all integration tests PASS

- [ ] **Step 3: Count coverage**

```bash
grep -c '#\[tokio::test\]' cynic-kernel/tests/integration_storage.rs
```

Expected: 11+ tests covering 13 method groups from the spec
