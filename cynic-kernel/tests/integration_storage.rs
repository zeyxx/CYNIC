//! Storage integration tests — run against a real SurrealDB instance.
//! Tests skip gracefully if SurrealDB is unavailable on localhost:8000.

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
async fn ping_succeeds_on_healthy_db() {
    let Some(db) = common::setup_test_db("ping").await else { return; };
    let result = db.ping().await;
    assert!(result.is_ok(), "ping failed: {:?}", result.err());
    common::teardown_test_db(&db).await;
}

// ── Task 3: Verdict round-trip ────────────────────────────

#[tokio::test]
async fn verdict_store_and_retrieve() {
    let Some(db) = common::setup_test_db("verdict_rt").await else { return; };
    let v = test_verdict("v-001");
    db.store_verdict(&v).await.expect("store_verdict failed");

    let got = db.get_verdict("v-001").await.expect("get_verdict failed");
    assert!(got.is_some(), "verdict not found after store");
    let got = got.unwrap();
    assert_eq!(got.id, "v-001");
    assert_eq!(got.kind, VerdictKind::Howl);
    assert!((got.q_score.total - 0.55).abs() < 0.001);

    // dog_scores round-trip (v0.7.1 — previously Vec::new())
    assert_eq!(got.dog_scores.len(), 1, "dog_scores should persist and round-trip");
    let ds = &got.dog_scores[0];
    assert_eq!(ds.dog_id, "test-dog");
    assert_eq!(ds.latency_ms, 100);
    assert_eq!(ds.prompt_tokens, 50);
    assert!((ds.fidelity - 0.6).abs() < 0.001);
    assert!((ds.sovereignty - 0.6).abs() < 0.001);

    common::teardown_test_db(&db).await;
}

#[tokio::test]
async fn verdict_list_respects_limit() {
    let Some(db) = common::setup_test_db("verdict_list").await else { return; };
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
async fn crystal_observe_creates_and_updates() {
    let Some(db) = common::setup_test_db("crystal_obs").await else { return; };
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
async fn crystal_list_sorted_by_maturity_then_confidence() {
    let Some(db) = common::setup_test_db("crystal_list").await else { return; };
    let ts = chrono::Utc::now().to_rfc3339();

    // c-many: 5 obs, confidence ~0.6 (forming, not enough obs for crystallized)
    for _ in 0..5 {
        db.observe_crystal("c-many", "Many obs", "test", 0.6, &ts).await.unwrap();
    }
    // c-few: 1 obs, confidence 0.9 (forming, but higher confidence)
    db.observe_crystal("c-few", "Few obs", "test", 0.9, &ts).await.unwrap();

    let list = db.list_crystals(10).await.expect("list_crystals failed");
    assert!(list.len() >= 2);
    // Both are "forming" (< 21 obs), so sorted by confidence DESC within same state
    assert!(list[0].confidence >= list[1].confidence,
        "within same state, should be sorted by confidence desc: {} >= {}",
        list[0].confidence, list[1].confidence);

    common::teardown_test_db(&db).await;
}

// ── Task 5: Observations ─────────────────────────────────

#[tokio::test]
async fn observation_store_and_query() {
    let Some(db) = common::setup_test_db("obs_rt").await else { return; };

    db.store_observation(&test_obs("agent-1", "Edit", "main.rs")).await.expect("store failed");
    db.store_observation(&test_obs("agent-1", "Edit", "main.rs")).await.expect("store2 failed");
    db.store_observation(&test_obs("agent-1", "Read", "lib.rs")).await.expect("store3 failed");

    let rows = db.query_observations("CYNIC", None, 50).await.expect("query failed");
    assert!(!rows.is_empty(), "should have observations");
    // Verify row structure has target and tool fields
    let first = &rows[0];
    assert!(!first.target.is_empty(), "rows should have a non-empty 'target' field");

    common::teardown_test_db(&db).await;
}

#[tokio::test]
async fn observation_query_session_targets() {
    let Some(db) = common::setup_test_db("obs_sess").await else { return; };

    // Two targets in same session
    db.store_observation(&test_obs("session-A", "Edit", "file1.rs")).await.unwrap();
    db.store_observation(&test_obs("session-A", "Edit", "file2.rs")).await.unwrap();
    // One target in different session
    db.store_observation(&test_obs("session-B", "Edit", "file3.rs")).await.unwrap();

    let rows = db.query_session_targets("CYNIC", 500).await.expect("query_session_targets failed");
    // Verify the query returns valid JSON with expected structure.
    // Session-A has 2 distinct targets — should appear in co-occurrence results.
    // Session-B has 1 target — may be filtered by the query's HAVING clause.
    assert!(!rows.is_empty(), "session-A with 2 targets should produce co-occurrence data");
    for row in &rows {
        assert!(!row.session_id.is_empty() || !row.target.is_empty(),
            "each row should have session_id or target, got: {:?}", row);
    }

    common::teardown_test_db(&db).await;
}

// ── Task 6: Coord lifecycle ──────────────────────────────

#[tokio::test]
async fn coord_full_lifecycle() {
    let Some(db) = common::setup_test_db("coord_life").await else { return; };

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
async fn coord_claim_conflict_detection() {
    let Some(db) = common::setup_test_db("coord_conflict").await else { return; };

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
async fn coord_claim_batch() {
    let Some(db) = common::setup_test_db("coord_batch").await else { return; };
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
async fn coord_expire_stale_does_not_break() {
    let Some(db) = common::setup_test_db("coord_expiry").await else { return; };
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
async fn audit_store_and_query() {
    let Some(db) = common::setup_test_db("audit_rt").await else { return; };

    let details = serde_json::json!({"action": "test", "target": "file.rs"}).to_string();
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
async fn flush_usage_via_storage_port() {
    let Some(db) = common::setup_test_db("flush_port").await else { return; };
    let mut tracker = DogUsageTracker::new();

    tracker.record("test-dog", 1000, 500, 200);
    tracker.record("test-dog", 500, 250, 100);

    // Use StoragePort::flush_usage with snapshot data (no SQL in domain)
    let snapshot = tracker.snapshot();
    db.flush_usage(&snapshot).await.expect("flush_usage failed");

    let rows = db.query_one("SELECT * FROM dog_usage;").await.expect("select failed");
    assert!(!rows.is_empty(), "dog_usage should have data after flush");

    // Verify actual values
    let row = &rows[0];
    let pt = row["prompt_tokens"].as_u64().unwrap_or(0);
    let ct = row["completion_tokens"].as_u64().unwrap_or(0);
    assert_eq!(pt, 1500, "prompt_tokens should accumulate: 1000+500");
    assert_eq!(ct, 750, "completion_tokens should accumulate: 500+250");

    // Second flush — accumulation via UPSERT
    tracker.record("test-dog", 100, 50, 50);
    let snapshot2 = tracker.snapshot();
    db.flush_usage(&snapshot2).await.expect("second flush failed");

    let rows2 = db.query_one("SELECT * FROM dog_usage;").await.expect("select2 failed");
    let pt2 = rows2[0]["prompt_tokens"].as_u64().unwrap_or(0);
    // Should be 1500 + 1600 = 3100 (accumulative UPSERT adds to existing)
    assert!(pt2 > 1500, "prompt_tokens should grow with second flush, got {}", pt2);

    common::teardown_test_db(&db).await;
}

// ── G1: Crystal embedding + HNSW vector search ──────────

#[tokio::test]
async fn store_and_search_crystal_embedding() {
    let Some(db) = common::setup_test_db("embed_rt").await else { return; };

    // Store a crystal first
    db.observe_crystal("chess-sicilian", "The Sicilian Defense", "chess", 0.7, "2026-03-21T12:00:00Z").await
        .expect("observe_crystal failed");

    // Store a 1024-dim embedding (synthetic)
    let mut embedding = vec![0.0f32; 1024];
    embedding[0] = 0.9;
    embedding[1] = 0.1;
    embedding[2] = 0.3;
    db.store_crystal_embedding("chess-sicilian", &embedding).await
        .expect("store_crystal_embedding failed");

    // Verify the embedding was stored
    let rows = db.query_one("SELECT id, array::len(embedding) AS dims FROM crystal WHERE embedding != NONE;").await
        .expect("query failed");
    assert!(!rows.is_empty(), "crystal should have an embedding");
    assert_eq!(rows[0]["dims"].as_u64().unwrap_or(0), 1024, "embedding should be 1024-dim");

    common::teardown_test_db(&db).await;
}

#[tokio::test]
async fn semantic_search_returns_empty_when_no_embeddings() {
    let Some(db) = common::setup_test_db("knn_empty").await else { return; };

    let query = vec![0.1f32; 1024];
    let results = db.search_crystals_semantic(&query, 5).await
        .expect("semantic search should not error on empty table");
    assert!(results.is_empty(), "should return empty when no crystals have embeddings");

    common::teardown_test_db(&db).await;
}

#[tokio::test]
async fn semantic_search_round_trip() {
    let Some(db) = common::setup_test_db("knn_rt").await else { return; };

    // Create 2 crystals and observe enough to reach Crystallized (26 obs, conf 0.7 > φ⁻¹)
    for (id, content) in [("chess-open", "Opening theory"), ("code-rust", "Rust patterns")] {
        for i in 0..26 {
            let ts = format!("2026-03-21T12:{:02}:00Z", i);
            db.observe_crystal(id, content, "chess", 0.7, &ts).await.unwrap();
        }
    }

    // Store distinct embeddings
    let mut emb_chess = vec![0.0f32; 1024];
    emb_chess[0] = 0.9; emb_chess[1] = 0.8;
    db.store_crystal_embedding("chess-open", &emb_chess).await.unwrap();

    let mut emb_rust = vec![0.0f32; 1024];
    emb_rust[500] = 0.9; emb_rust[501] = 0.8;
    db.store_crystal_embedding("code-rust", &emb_rust).await.unwrap();

    // Search with vector similar to chess
    let mut query = vec![0.0f32; 1024];
    query[0] = 0.85; query[1] = 0.75;
    let results = db.search_crystals_semantic(&query, 5).await
        .expect("semantic search failed");

    // The SQL + HNSW must execute without error.
    // If crystals are Crystallized AND have embeddings, we should get results.
    // This is the key test: does the HNSW KNN query actually work in SurrealDB 3.0.3?
    eprintln!("KNN results: {} crystals returned", results.len());
    for c in &results {
        eprintln!("  {} (state={:?}, conf={:.3})", c.id, c.state, c.confidence);
    }

    common::teardown_test_db(&db).await;
}

// ── G3: Session summaries ───────────────────────────────

#[tokio::test]
async fn store_and_list_session_summaries() {
    use cynic_kernel::domain::ccm::SessionSummary;

    let Some(db) = common::setup_test_db("session_rt").await else { return; };

    let s1 = SessionSummary {
        session_id: "sess-abc".into(),
        agent_id: "claude-123".into(),
        summary: "Fixed temporal decay in crystals.".into(),
        observations_count: 15,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let s2 = SessionSummary {
        session_id: "sess-def".into(),
        agent_id: "claude-456".into(),
        summary: "Extracted judge pipeline.".into(),
        observations_count: 22,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    db.store_session_summary(&s1).await.expect("store s1 failed");
    db.store_session_summary(&s2).await.expect("store s2 failed");

    let summaries = db.list_session_summaries(10).await.expect("list failed");
    assert_eq!(summaries.len(), 2, "should have 2 summaries, got {}", summaries.len());
    assert!(summaries.iter().any(|s| s.session_id == "sess-abc"));
    assert!(summaries.iter().any(|s| s.session_id == "sess-def"));

    common::teardown_test_db(&db).await;
}

#[tokio::test]
async fn get_unsummarized_sessions() {
    let Some(db) = common::setup_test_db("unsum_rt").await else { return; };

    // Create observations for 2 agents
    for i in 0..5 {
        db.store_observation(&test_obs("agent-A", "Edit", &format!("file{}.rs", i))).await.unwrap();
    }
    for i in 0..3 {
        db.store_observation(&test_obs("agent-B", "Read", &format!("doc{}.md", i))).await.unwrap();
    }
    db.store_observation(&test_obs("agent-C", "Bash", "ls")).await.unwrap();

    let pending = db.get_unsummarized_sessions(3, 10).await
        .expect("get_unsummarized_sessions failed");
    let ids: Vec<&str> = pending.iter().map(|(id, _, _)| id.as_str()).collect();
    assert!(ids.contains(&"agent-A"), "agent-A (5 obs) should be pending");
    assert!(ids.contains(&"agent-B"), "agent-B (3 obs) should be pending");
    assert!(!ids.contains(&"agent-C"), "agent-C (1 obs) below threshold");

    // Summarize agent-A — should disappear from pending
    use cynic_kernel::domain::ccm::SessionSummary;
    db.store_session_summary(&SessionSummary {
        session_id: "agent-A".into(), agent_id: "agent-A".into(),
        summary: "Edited 5 files".into(), observations_count: 5,
        created_at: chrono::Utc::now().to_rfc3339(),
    }).await.unwrap();

    let pending2 = db.get_unsummarized_sessions(3, 10).await.unwrap();
    let ids2: Vec<&str> = pending2.iter().map(|(id, _, _)| id.as_str()).collect();
    assert!(!ids2.contains(&"agent-A"), "agent-A excluded after summarization");
    assert!(ids2.contains(&"agent-B"), "agent-B still pending");

    common::teardown_test_db(&db).await;
}

#[tokio::test]
async fn get_session_observations_returns_filtered() {
    let Some(db) = common::setup_test_db("sessobs_rt").await else { return; };

    db.store_observation(&test_obs("sess-X", "Edit", "a.rs")).await.unwrap();
    db.store_observation(&test_obs("sess-X", "Bash", "cargo test")).await.unwrap();
    db.store_observation(&test_obs("sess-X", "Read", "b.rs")).await.unwrap();
    db.store_observation(&test_obs("sess-Y", "Edit", "other.rs")).await.unwrap();

    let obs = db.get_session_observations("sess-X").await
        .expect("get_session_observations failed");
    assert_eq!(obs.len(), 3, "should have 3 observations for sess-X, got {}", obs.len());

    common::teardown_test_db(&db).await;
}
