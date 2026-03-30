#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Route-level integration tests — exercise the Axum router with mock dependencies.
//! Tests middleware (auth, rate limiting), handler wiring, and response formatting.
//! No running server needed — uses tower::ServiceExt::oneshot.

use std::sync::Arc;

use axum::body::Body;
use http_body_util::BodyExt;
use tower::ServiceExt;

use cynic_kernel::api::rest::{self, AppState, PerIpRateLimiter, StorageInfo};
use cynic_kernel::dogs::deterministic::DeterministicDog;
use cynic_kernel::domain::coord::NullCoord;
use cynic_kernel::domain::embedding::NullEmbedding;
use cynic_kernel::domain::metrics::Metrics;
use cynic_kernel::domain::storage::NullStorage;
use cynic_kernel::domain::usage::DogUsageTracker;
use cynic_kernel::domain::verdict_cache::VerdictCache;
use cynic_kernel::infra::task_health::TaskHealth;
use cynic_kernel::judge::Judge;

fn test_state(api_key: Option<&str>) -> Arc<AppState> {
    let dogs: Vec<Box<dyn cynic_kernel::domain::dog::Dog>> = vec![Box::new(DeterministicDog)];
    let breakers: Vec<Arc<dyn cynic_kernel::domain::health_gate::HealthGate>> = dogs
        .iter()
        .map(|d| {
            Arc::new(cynic_kernel::infra::circuit_breaker::CircuitBreaker::new(
                d.id().to_string(),
            )) as Arc<dyn cynic_kernel::domain::health_gate::HealthGate>
        })
        .collect();
    let judge = Arc::new(Judge::new(dogs, breakers));
    Arc::new(AppState {
        judge,
        storage: Arc::new(NullStorage),
        coord: Arc::new(NullCoord),
        embedding: Arc::new(NullEmbedding),
        usage: Arc::new(tokio::sync::Mutex::new(DogUsageTracker::new())),
        verdict_cache: Arc::new(VerdictCache::new()),
        task_health: Arc::new(TaskHealth::new()),
        metrics: Arc::new(Metrics::new()),
        api_key: api_key.map(|s| s.to_string()),
        storage_info: StorageInfo {
            namespace: "test".into(),
            database: "test".into(),
        },
        rate_limiter: PerIpRateLimiter::new(100),
        judge_limiter: PerIpRateLimiter::new(100),
        ready_cache: cynic_kernel::api::rest::ReadyCache::new(),
        bg_semaphore: std::sync::Arc::new(tokio::sync::Semaphore::new(64)),
        bg_tasks: tokio_util::task::TaskTracker::new(),
        sse_semaphore: std::sync::Arc::new(tokio::sync::Semaphore::new(32)),
        introspection_alerts: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        event_tx: tokio::sync::broadcast::channel(16).0,
        chain_verified: std::sync::atomic::AtomicBool::new(true),
        environment: std::sync::Arc::new(std::sync::RwLock::new(None)),
    })
}

async fn body_json(body: Body) -> serde_json::Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

// ── /health ─────────────────────────────────────────────────

#[tokio::test]
async fn health_no_auth_returns_public_info() {
    let state = test_state(Some("secret-key"));
    let app = rest::router(state);

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 503 expected: test state has 1 Dog + NullStorage (ping fails → critical)
    // Industry standard: 503 = degraded/critical, 200 = sovereign
    assert!(
        resp.status() == 200 || resp.status() == 503,
        "CONTRACT: health must return 200 or 503, got {}",
        resp.status()
    );
    let v = body_json(resp.into_body()).await;
    // Public: has status, version, phi_max — but NOT dog_count or dogs array (attack surface)
    assert!(v["status"].is_string());
    assert!(v["version"].is_string());
    assert!(v["phi_max"].is_number());
    assert!(
        v.get("dog_count").is_none(),
        "Public health should not expose dog_count"
    );
    assert!(
        v.get("dogs").is_none(),
        "Public health should not expose dog details"
    );
}

#[tokio::test]
async fn health_with_auth_returns_full_details() {
    let state = test_state(Some("secret-key"));
    let app = rest::router(state);

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/health")
                .header("Authorization", "Bearer secret-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 503 expected in test: 1 Dog + NullStorage → critical
    assert!(
        resp.status() == 200 || resp.status() == 503,
        "CONTRACT: health must return 200 or 503"
    );
    let v = body_json(resp.into_body()).await;

    // ── API CONTRACT: authenticated /health response ──
    // Every field listed here is consumed by at least one external system.
    // Removing any field without updating this test = compile-time catch.
    // Consumer map:
    //   dogs[]           → healthcheck.sh (dog count), /status skill, frontend
    //   dogs[].circuit   → healthcheck.sh (healthy count), remediation watcher
    //   dogs[].failures  → /status skill, frontend dashboard
    //   status           → healthcheck.sh, frontend, load balancer
    //   storage          → healthcheck.sh, /status skill
    //   version          → /status skill, deploy verification
    //   total_requests   → /status skill, cost tracking
    //   total_tokens     → cost tracking, /usage
    //   estimated_cost_usd → /status skill, cost dashboard
    //   uptime_seconds   → /status skill
    //   embedding        → /status skill
    //   storage_metrics  → /status skill (queries, errors, latency)

    // Core fields (MUST exist)
    assert!(v["status"].is_string(), "CONTRACT: status must be string");
    assert!(v["version"].is_string(), "CONTRACT: version must be string");
    assert!(v["dogs"].is_array(), "CONTRACT: dogs must be array");
    assert!(v["storage"].is_string(), "CONTRACT: storage must be string");
    assert!(
        v["total_requests"].is_number(),
        "CONTRACT: total_requests must be number"
    );
    assert!(
        v["total_tokens"].is_number(),
        "CONTRACT: total_tokens must be number"
    );
    assert!(
        v["estimated_cost_usd"].is_number(),
        "CONTRACT: estimated_cost_usd must be number"
    );
    assert!(
        v["uptime_seconds"].is_number(),
        "CONTRACT: uptime_seconds must be number"
    );

    // Dog structure (MUST have these fields per dog)
    let dogs = v["dogs"].as_array().unwrap();
    assert!(!dogs.is_empty(), "CONTRACT: at least 1 dog");
    let dog = &dogs[0];
    assert!(dog["id"].is_string(), "CONTRACT: dog.id must be string");
    assert!(
        dog["circuit"].is_string(),
        "CONTRACT: dog.circuit must be string"
    );
    assert!(
        dog["failures"].is_number(),
        "CONTRACT: dog.failures must be number"
    );
    assert!(dog["kind"].is_string(), "CONTRACT: dog.kind must be string");

    // Fields that MUST NOT be in public but MUST be in authenticated
    assert!(
        v.get("storage_metrics").is_some(),
        "CONTRACT: storage_metrics must exist in auth response"
    );
}

// ── /dogs ───────────────────────────────────────────────────

#[tokio::test]
async fn dogs_requires_auth() {
    let state = test_state(Some("secret-key"));
    let app = rest::router(state);

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/dogs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn dogs_with_auth_returns_list() {
    let state = test_state(Some("secret-key"));
    let app = rest::router(state);

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/dogs")
                .header("Authorization", "Bearer secret-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let v = body_json(resp.into_body()).await;
    let dogs = v.as_array().unwrap();
    assert_eq!(dogs.len(), 1);
    assert_eq!(dogs[0], "deterministic-dog");
}

// ── /judge ──────────────────────────────────────────────────

#[tokio::test]
async fn judge_requires_auth() {
    let state = test_state(Some("secret-key"));
    let app = rest::router(state);

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/judge")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"content":"test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn judge_produces_verdict() {
    let state = test_state(Some("key"));
    let app = rest::router(state);

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/judge")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer key")
                .body(Body::from(
                    r#"{"content":"The Sicilian Defense is strong","domain":"chess"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let v = body_json(resp.into_body()).await;

    // ── API CONTRACT: /judge response ──
    // Consumer map:
    //   verdict_id     → frontend (link to detail), MCP cynic_judge
    //   verdict        → frontend badge, MCP, CCM crystal domain
    //   q_score.total  → frontend score display, MCP, CCM crystal confidence
    //   q_score.{axiom} → frontend radar chart, MCP per-axiom breakdown
    //   reasoning.{axiom} → frontend reasoning display, MCP
    //   dogs_used      → frontend dog list, /status skill
    //   stimulus_summary → CCM crystal content, verdict history
    assert!(
        v["verdict_id"].is_string(),
        "CONTRACT: verdict_id must be string"
    );
    let verdict = v["verdict"]
        .as_str()
        .expect("CONTRACT: verdict must be string");
    assert!(
        ["Howl", "Wag", "Growl", "Bark"].contains(&verdict),
        "CONTRACT: verdict must be Howl/Wag/Growl/Bark"
    );

    // Q-Score structure
    let q = &v["q_score"];
    assert!(
        q["total"].is_number(),
        "CONTRACT: q_score.total must be number"
    );
    assert!(
        q["fidelity"].is_number(),
        "CONTRACT: q_score.fidelity must be number"
    );
    assert!(q["phi"].is_number(), "CONTRACT: q_score.phi must be number");
    assert!(
        q["verify"].is_number(),
        "CONTRACT: q_score.verify must be number"
    );
    assert!(
        q["culture"].is_number(),
        "CONTRACT: q_score.culture must be number"
    );
    assert!(
        q["burn"].is_number(),
        "CONTRACT: q_score.burn must be number"
    );
    assert!(
        q["sovereignty"].is_number(),
        "CONTRACT: q_score.sovereignty must be number"
    );

    // Reasoning structure
    let r = &v["reasoning"];
    assert!(
        r["fidelity"].is_string(),
        "CONTRACT: reasoning.fidelity must be string"
    );

    // Dogs
    assert!(
        v["dogs_used"].is_string(),
        "CONTRACT: dogs_used must be string"
    );
    // dog_scores — per-Dog breakdown (v0.7.1: persisted to DB, round-trips correctly)
    assert!(
        v["dog_scores"].is_array(),
        "CONTRACT: dog_scores must be array"
    );
    let scores = v["dog_scores"].as_array().unwrap();
    assert!(
        !scores.is_empty(),
        "CONTRACT: dog_scores must have at least 1 entry (deterministic-dog)"
    );
    let ds = &scores[0];
    assert!(
        ds["dog_id"].is_string(),
        "CONTRACT: dog_scores[].dog_id must be string"
    );
    assert!(
        ds["fidelity"].is_number(),
        "CONTRACT: dog_scores[].fidelity must be number"
    );
    assert!(
        ds["sovereignty"].is_number(),
        "CONTRACT: dog_scores[].sovereignty must be number"
    );
}

// ── /health without auth config (open API) ──────────────────

#[tokio::test]
async fn open_api_health_returns_full_details() {
    let state = test_state(None); // No API key = open
    let app = rest::router(state);

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        resp.status() == 200 || resp.status() == 503,
        "CONTRACT: health must return 200 or 503"
    );
    let v = body_json(resp.into_body()).await;
    // Open API: everyone gets full details
    assert!(v["dogs"].is_array());
}

// ── /verdicts with null storage ─────────────────────────────

#[tokio::test]
async fn verdicts_with_null_storage_returns_500() {
    let state = test_state(Some("key"));
    let app = rest::router(state);

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/verdicts")
                .header("Authorization", "Bearer key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // NullStorage errors on list_verdicts — handler should return 500
    assert_eq!(resp.status(), 500);
}

// ── Crystal CRUD ────────────────────────────────────────────

#[tokio::test]
async fn create_crystal_requires_auth() {
    let state = test_state(Some("key"));
    let app = rest::router(state);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/crystal")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"content":"test insight","domain":"chess"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn create_crystal_returns_500_with_null_storage() {
    // RC5: NullStorage now returns Err — honest about unavailability
    let state = test_state(Some("key"));
    let app = rest::router(state);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/crystal")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer key")
                .body(Body::from(
                    r#"{"content":"The Sicilian Defense is strong","domain":"chess"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        500,
        "NullStorage: POST /crystal must return 500 (storage unavailable)"
    );
}

#[tokio::test]
async fn create_crystal_rejects_empty_content() {
    let state = test_state(Some("key"));
    let app = rest::router(state);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/crystal")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer key")
                .body(Body::from(r#"{"content":"","domain":"chess"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        400,
        "CONTRACT: empty content must be rejected"
    );
}

#[tokio::test]
async fn delete_crystal_returns_500_with_null_storage() {
    // RC5: NullStorage now returns Err — honest about unavailability
    let state = test_state(Some("key"));
    let app = rest::router(state);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("DELETE")
                .uri("/crystal/test-id")
                .header("Authorization", "Bearer key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        500,
        "NullStorage: DELETE /crystal must return 500"
    );
}

#[tokio::test]
async fn observe_crystal_endpoint_removed() {
    // Endpoint removed: crystal observations flow through the judge pipeline only.
    // Direct REST observation always failed (voter_count=0 → quorum rejection).
    let state = test_state(Some("key"));
    let app = rest::router(state);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/crystal/test-id/observe")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer key")
                .body(Body::from(
                    r#"{"content":"test","domain":"chess","score":0.85}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        405,
        "POST /crystal/id/observe should be gone (method not allowed)"
    );
}

#[tokio::test]
async fn list_crystals_returns_500_with_null_storage() {
    // RC5: NullStorage now returns Err — honest about unavailability
    let state = test_state(Some("key"));
    let app = rest::router(state);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/crystals?domain=chess&state=crystallized")
                .header("Authorization", "Bearer key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        500,
        "NullStorage: GET /crystals must return 500"
    );
}

// ── Gate 3 serialization contracts ─────────────────────────
// These tests verify that domain structs serialize to the expected JSON shape.
// They exist because Gate 3 replaced Vec<serde_json::Value> with typed structs.
// A field rename or removal here would be a breaking API change.

#[test]
fn raw_observation_json_shape() {
    use cynic_kernel::domain::storage::RawObservation;
    let obs = RawObservation {
        id: "observation:abc123".into(),
        tool: "Edit".into(),
        target: "main.rs".into(),
        domain: "code".into(),
        status: "success".into(),
        context: "test".into(),
        created_at: "2026-03-23T00:00:00Z".into(),
        project: "CYNIC".into(),
        agent_id: "agent-1".into(),
        session_id: "sess-1".into(),
    };
    let v: serde_json::Value = serde_json::to_value(&obs).unwrap();
    // CONTRACT: these fields must exist with these exact names
    for field in [
        "id",
        "tool",
        "target",
        "domain",
        "status",
        "context",
        "created_at",
        "project",
        "agent_id",
        "session_id",
    ] {
        assert!(
            v.get(field).is_some(),
            "CONTRACT: RawObservation must have '{field}' field"
        );
    }
}

#[test]
fn audit_entry_json_shape() {
    use cynic_kernel::domain::coord::AuditEntry;
    let entry = AuditEntry {
        id: "mcp_audit:xyz".into(),
        ts: "2026-03-23T00:00:00Z".into(),
        tool: "cynic_judge".into(),
        agent_id: "agent-1".into(),
        details: r#"{"action":"test"}"#.into(),
    };
    let v: serde_json::Value = serde_json::to_value(&entry).unwrap();
    for field in ["id", "ts", "tool", "agent_id", "details"] {
        assert!(
            v.get(field).is_some(),
            "CONTRACT: AuditEntry must have '{field}' field"
        );
    }
}

#[test]
fn agent_info_json_shape() {
    use cynic_kernel::domain::coord::AgentInfo;
    let info = AgentInfo {
        id: "agent_session:abc".into(),
        agent_id: "claude-123".into(),
        agent_type: "claude".into(),
        intent: "testing".into(),
        active: true,
        registered_at: "2026-03-23T00:00:00Z".into(),
        last_seen: "2026-03-23T00:00:00Z".into(),
    };
    let v: serde_json::Value = serde_json::to_value(&info).unwrap();
    for field in [
        "id",
        "agent_id",
        "agent_type",
        "intent",
        "active",
        "registered_at",
        "last_seen",
    ] {
        assert!(
            v.get(field).is_some(),
            "CONTRACT: AgentInfo must have '{field}' field"
        );
    }
}

#[test]
fn claim_entry_json_shape() {
    use cynic_kernel::domain::coord::ClaimEntry;
    let claim = ClaimEntry {
        id: "work_claim:abc".into(),
        agent_id: "claude-123".into(),
        target: "main.rs".into(),
        claim_type: "file".into(),
        active: true,
        claimed_at: "2026-03-23T00:00:00Z".into(),
    };
    let v: serde_json::Value = serde_json::to_value(&claim).unwrap();
    for field in [
        "id",
        "agent_id",
        "target",
        "claim_type",
        "active",
        "claimed_at",
    ] {
        assert!(
            v.get(field).is_some(),
            "CONTRACT: ClaimEntry must have '{field}' field"
        );
    }
}

#[test]
fn usage_row_json_shape() {
    use cynic_kernel::domain::storage::UsageRow;
    let row = UsageRow {
        dog_id: "gemini".into(),
        prompt_tokens: 1000,
        completion_tokens: 500,
        requests: 10,
        failures: 1,
        total_latency_ms: 5000,
    };
    let v: serde_json::Value = serde_json::to_value(&row).unwrap();
    for field in [
        "dog_id",
        "prompt_tokens",
        "completion_tokens",
        "requests",
        "failures",
        "total_latency_ms",
    ] {
        assert!(
            v.get(field).is_some(),
            "CONTRACT: UsageRow must have '{field}' field"
        );
    }
}

// ── F13 regression: CJK byte/char mismatch ────────────────────────

#[tokio::test]
async fn judge_cjk_content_counts_chars_not_bytes() {
    // 1000 CJK chars = 3000 bytes. Must be accepted (limit is 4000 chars).
    // Before fix: .len() saw 3000 bytes, worked but would reject at ~1333 CJK chars.
    let cjk_content: String = std::iter::repeat('漢').take(1000).collect();
    assert_eq!(
        cjk_content.len(),
        3000,
        "precondition: 3 bytes per CJK char"
    );
    assert_eq!(
        cjk_content.chars().count(),
        1000,
        "precondition: 1000 chars"
    );

    let state = test_state(Some("key"));
    let app = rest::router(state);

    let body = serde_json::json!({"content": cjk_content, "domain": "test"});
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/judge")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer key")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        200,
        "F13: CJK content within char limit must be accepted (was rejected by byte count)"
    );
}

// ── F22 regression: /ready caches DB ping ────────────────────────

#[tokio::test]
async fn ready_cache_returns_cached_value_within_ttl() {
    use cynic_kernel::api::rest::ReadyCache;

    let cache = ReadyCache::new();
    // First call: stale (checked_at=0)
    assert!(cache.get().is_none(), "precondition: new cache is stale");
    // Set to ok
    cache.set(true);
    assert_eq!(
        cache.get(),
        Some(true),
        "F22: cache returns true within TTL"
    );
    // Set to not-ok
    cache.set(false);
    assert_eq!(cache.get(), Some(false), "F22: cache reflects latest set()");
}

// ── RC1-6 regression: coord input validation ─────────────────────

#[tokio::test]
async fn coord_register_rejects_oversized_intent() {
    let state = test_state(Some("key"));
    let app = rest::router(state);

    let long_intent: String = std::iter::repeat('x').take(501).collect();
    let body = serde_json::json!({"agent_id": "test", "intent": long_intent});
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/coord/register")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer key")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        400,
        "RC1-6: oversized intent must be rejected"
    );
}

#[tokio::test]
async fn coord_register_accepts_valid_intent() {
    let state = test_state(Some("key"));
    let app = rest::router(state);

    let body = serde_json::json!({"agent_id": "test-agent", "intent": "implementing feature X"});
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/coord/register")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer key")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // NullCoord returns Ok(()) for register — should succeed
    assert_eq!(resp.status(), 200, "RC1-6: valid intent must be accepted");
}

// ── F23 regression: SSE connection limit ─────────────────────────

#[tokio::test]
async fn events_rejects_when_sse_semaphore_exhausted() {
    // Build state with 0 SSE permits — simulates all 32 slots taken.
    let dogs: Vec<Box<dyn cynic_kernel::domain::dog::Dog>> = vec![Box::new(DeterministicDog)];
    let breakers: Vec<Arc<dyn cynic_kernel::domain::health_gate::HealthGate>> = dogs
        .iter()
        .map(|d| {
            Arc::new(cynic_kernel::infra::circuit_breaker::CircuitBreaker::new(
                d.id().to_string(),
            )) as Arc<dyn cynic_kernel::domain::health_gate::HealthGate>
        })
        .collect();
    let judge = Arc::new(cynic_kernel::judge::Judge::new(dogs, breakers));
    let state = Arc::new(AppState {
        judge,
        storage: Arc::new(NullStorage),
        coord: Arc::new(NullCoord),
        embedding: Arc::new(NullEmbedding),
        usage: Arc::new(tokio::sync::Mutex::new(DogUsageTracker::new())),
        verdict_cache: Arc::new(VerdictCache::new()),
        task_health: Arc::new(TaskHealth::new()),
        metrics: Arc::new(Metrics::new()),
        api_key: None,
        storage_info: StorageInfo {
            namespace: "test".into(),
            database: "test".into(),
        },
        rate_limiter: PerIpRateLimiter::new(100),
        judge_limiter: PerIpRateLimiter::new(100),
        ready_cache: cynic_kernel::api::rest::ReadyCache::new(),
        bg_semaphore: std::sync::Arc::new(tokio::sync::Semaphore::new(64)),
        bg_tasks: tokio_util::task::TaskTracker::new(),
        sse_semaphore: std::sync::Arc::new(tokio::sync::Semaphore::new(0)), // F23: zero permits
        introspection_alerts: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        event_tx: tokio::sync::broadcast::channel(16).0,
        chain_verified: std::sync::atomic::AtomicBool::new(true),
        environment: std::sync::Arc::new(std::sync::RwLock::new(None)),
    });
    let app = rest::router(state);

    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        503,
        "F23: /events must return 503 when SSE connection limit reached"
    );
}

// ── I2 regression: control chars in agent_id ─────────────────────

#[tokio::test]
async fn coord_register_rejects_control_chars_in_agent_id() {
    let state = test_state(Some("key"));
    let app = rest::router(state);

    let body = serde_json::json!({"agent_id": "test\x00admin", "intent": "probe"});
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/coord/register")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer key")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        400,
        "I2: control characters in agent_id must be rejected"
    );
}
