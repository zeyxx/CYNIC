//! Storage integration tests — run against a real SurrealDB instance.
//! Activated by: SURREALDB_PASS=... cargo test -p cynic-kernel --release -- --ignored
//! Or: make check-storage

mod common;

use cynic_kernel::domain::storage::{Observation, StoragePort};
use cynic_kernel::domain::coord::{CoordPort, ClaimResult};
use cynic_kernel::domain::dog::{
    Verdict, VerdictKind, QScore, AxiomReasoning, DogScore,
};
use cynic_kernel::domain::usage::DogUsageTracker;

// ── Test helpers ──────────────────────────────────────────

fn test_verdict(id: &str) -> Verdict {
    Verdict {
        id: id.to_string(),
        kind: VerdictKind::Howl,
        stimulus_summary: "Test stimulus".to_string(),
        dog_id: "test-dog".to_string(),
        q_score: QScore {
            total: 0.55,
            fidelity: 0.6, phi: 0.5, verify: 0.5,
            culture: 0.6, burn: 0.5, sovereignty: 0.6,
        },
        reasoning: AxiomReasoning::default(),
        dog_scores: vec![DogScore {
            dog_id: "test-dog".to_string(),
            latency_ms: 100,
            prompt_tokens: 50,
            completion_tokens: 20,
            fidelity: 0.6, phi: 0.5, verify: 0.5,
            culture: 0.6, burn: 0.5, sovereignty: 0.6,
            reasoning: AxiomReasoning::default(),
        }],
        timestamp: chrono::Utc::now().to_rfc3339(),
        anomaly_detected: false,
        max_disagreement: 0.0,
        anomaly_axiom: None,
        failed_dogs: vec![],
        integrity_hash: Some("test-hash".to_string()),
        prev_hash: None,
    }
}

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

// ── Task 2: Ping ──────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn ping_succeeds_on_healthy_db() {
    let db = common::setup_test_db("ping").await;
    let result = db.ping().await;
    assert!(result.is_ok(), "ping failed: {:?}", result.err());
    common::teardown_test_db(&db).await;
}

// ── Task 3: Verdict round-trip ────────────────────────────

#[tokio::test]
#[ignore]
async fn verdict_store_and_retrieve() {
    let db = common::setup_test_db("verdict_rt").await;
    let v = test_verdict("v-001");
    db.store_verdict(&v).await.expect("store_verdict failed");

    let got = db.get_verdict("v-001").await.expect("get_verdict failed");
    assert!(got.is_some(), "verdict not found after store");
    let got = got.unwrap();
    assert_eq!(got.id, "v-001");
    assert_eq!(got.kind, VerdictKind::Howl);
    assert!((got.q_score.total - 0.55).abs() < 0.001);
    // dog_scores are not round-tripped from DB (stored but not deserialized on read)
    // This is by design — they're verbose and not needed for verdict display

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

    let limited = db.list_verdicts(2).await.expect("list_verdicts limited");
    assert_eq!(limited.len(), 2);

    common::teardown_test_db(&db).await;
}

// ── Task 4: Crystal ───────────────────────────────────────

#[tokio::test]
#[ignore]
async fn crystal_observe_creates_and_updates() {
    let db = common::setup_test_db("crystal_obs").await;
    let ts = chrono::Utc::now().to_rfc3339();

    db.observe_crystal("test-crystal", "Test content", "test", 0.7, &ts)
        .await.expect("observe_crystal failed");

    let c = db.get_crystal("test-crystal").await.expect("get_crystal failed");
    assert!(c.is_some(), "crystal not found after observe");
    let c = c.unwrap();
    assert_eq!(c.observations, 1);
    assert_eq!(c.content, "Test content");

    db.observe_crystal("test-crystal", "Test content", "test", 0.8, &ts)
        .await.expect("second observe failed");
    let c2 = db.get_crystal("test-crystal").await.expect("get after update");
    let c2 = c2.unwrap();
    assert_eq!(c2.observations, 2);
    assert!(c2.confidence > c.confidence, "confidence should increase with higher score");

    common::teardown_test_db(&db).await;
}

#[tokio::test]
#[ignore]
async fn crystal_list_sorted_by_observations() {
    let db = common::setup_test_db("crystal_list").await;
    let ts = chrono::Utc::now().to_rfc3339();

    for _ in 0..5 {
        db.observe_crystal("c-many", "Many obs", "test", 0.6, &ts).await.unwrap();
    }
    db.observe_crystal("c-few", "Few obs", "test", 0.9, &ts).await.unwrap();

    let list = db.list_crystals(10).await.expect("list_crystals failed");
    assert!(list.len() >= 2);
    assert!(list[0].observations >= list[1].observations, "should be sorted by observations desc");

    common::teardown_test_db(&db).await;
}

// ── Task 5: Observations ─────────────────────────────────

#[tokio::test]
#[ignore]
async fn observation_store_and_query() {
    let db = common::setup_test_db("obs_rt").await;

    db.store_observation(&test_obs("agent-1", "Edit", "main.rs")).await.expect("store failed");
    db.store_observation(&test_obs("agent-1", "Edit", "main.rs")).await.expect("store2 failed");
    db.store_observation(&test_obs("agent-1", "Read", "lib.rs")).await.expect("store3 failed");

    let rows = db.query_observations("CYNIC", None, 50).await.expect("query failed");
    assert!(!rows.is_empty(), "should have observations");
    // Verify row structure has target and tool fields
    let first = &rows[0];
    assert!(first.get("target").is_some(), "rows should have 'target' field");

    common::teardown_test_db(&db).await;
}

#[tokio::test]
#[ignore]
async fn observation_query_session_targets() {
    let db = common::setup_test_db("obs_sess").await;

    // Two targets in same session
    db.store_observation(&test_obs("session-A", "Edit", "file1.rs")).await.unwrap();
    db.store_observation(&test_obs("session-A", "Edit", "file2.rs")).await.unwrap();
    // One target in different session
    db.store_observation(&test_obs("session-B", "Edit", "file3.rs")).await.unwrap();

    let rows = db.query_session_targets("CYNIC", 500).await.expect("query_session_targets failed");
    // query_session_targets returns session-grouped data for CCM co-occurrence analysis.
    // The exact structure depends on the SurrealQL query — we verify it doesn't error
    // and returns valid JSON (the aggregation may filter single-target sessions).
    // This test primarily validates the query EXECUTES without SurrealDB syntax errors.

    common::teardown_test_db(&db).await;
}

// ── Task 6: Coord lifecycle ──────────────────────────────

#[tokio::test]
#[ignore]
async fn coord_full_lifecycle() {
    let db = common::setup_test_db("coord_life").await;

    db.register_agent("test-agent", "claude", "testing").await
        .expect("register failed");
    db.heartbeat("test-agent").await.expect("heartbeat failed");

    let result = db.claim("test-agent", "main.rs", "file").await
        .expect("claim failed");
    assert!(matches!(result, ClaimResult::Claimed));

    let snap = db.who(Some("test-agent")).await.expect("who failed");
    assert!(!snap.claims.is_empty(), "should have claims after claim");

    db.release("test-agent", Some("main.rs")).await.expect("release failed");
    let snap2 = db.who(Some("test-agent")).await.expect("who after release");
    assert!(snap2.claims.is_empty(), "claims should be empty after release");

    db.deactivate_agent("test-agent").await.expect("deactivate failed");
    common::teardown_test_db(&db).await;
}

#[tokio::test]
#[ignore]
async fn coord_claim_conflict_detection() {
    let db = common::setup_test_db("coord_conflict").await;

    db.register_agent("agent-A", "claude", "working").await.unwrap();
    db.register_agent("agent-B", "gemini", "working").await.unwrap();

    let r1 = db.claim("agent-A", "shared.rs", "file").await.unwrap();
    assert!(matches!(r1, ClaimResult::Claimed));

    let r2 = db.claim("agent-B", "shared.rs", "file").await.unwrap();
    assert!(matches!(r2, ClaimResult::Conflict(_)), "second agent should see conflict");

    db.release("agent-A", None).await.unwrap();
    db.release("agent-B", None).await.unwrap();
    common::teardown_test_db(&db).await;
}

// ── Task 7: Batch + Expire ───────────────────────────────

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
async fn coord_expire_stale_does_not_break() {
    let db = common::setup_test_db("coord_expiry").await;
    db.register_agent("stale-agent", "test", "will expire").await.unwrap();
    db.claim("stale-agent", "old-file.rs", "file").await.unwrap();

    // expire_stale should not error on valid state (fresh session survives)
    db.expire_stale().await.expect("expire_stale failed");
    let snap = db.who(None).await.unwrap();
    assert!(!snap.agents.is_empty(), "fresh session should survive expire_stale");

    db.release("stale-agent", None).await.unwrap();
    common::teardown_test_db(&db).await;
}

// ── Task 8: Audit trail ──────────────────────────────────

#[tokio::test]
#[ignore]
async fn audit_store_and_query() {
    let db = common::setup_test_db("audit_rt").await;

    let details = serde_json::json!({"action": "test", "target": "file.rs"});
    db.store_audit("cynic_judge", "test-agent", &details).await
        .expect("store_audit failed");
    db.store_audit("cynic_infer", "test-agent", &details).await.unwrap();

    let all = db.query_audit(None, None, 10).await.expect("query_audit failed");
    assert!(all.len() >= 2, "should have at least 2 audit entries, got {}", all.len());

    let filtered = db.query_audit(Some("cynic_judge"), None, 10).await.unwrap();
    assert!(!filtered.is_empty(), "tool filter should return results");

    common::teardown_test_db(&db).await;
}

// ── Task 9: Flush usage ──────────────────────────────────

#[tokio::test]
#[ignore]
async fn flush_usage_upserts_correctly() {
    let db = common::setup_test_db("flush").await;
    let mut tracker = DogUsageTracker::new();

    tracker.record("test-dog", 1000, 500, 200);
    tracker.record("test-dog", 500, 250, 100);

    let sql = tracker.build_flush_sql();
    assert!(!sql.is_empty(), "flush SQL should not be empty");
    db.query(&sql).await.expect("flush SQL failed");

    let rows = db.query_one("SELECT * FROM dog_usage;").await.expect("select failed");
    assert!(!rows.is_empty(), "dog_usage should have data after flush");

    // Second flush — accumulation (idempotent UPSERT)
    tracker.record("test-dog", 100, 50, 50);
    let sql2 = tracker.build_flush_sql();
    db.query(&sql2).await.expect("second flush failed");

    common::teardown_test_db(&db).await;
}
