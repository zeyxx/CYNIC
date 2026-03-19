//! Route-level integration tests — exercise the Axum router with mock dependencies.
//! Tests middleware (auth, rate limiting), handler wiring, and response formatting.
//! No running server needed — uses tower::ServiceExt::oneshot.

use std::sync::Arc;

use axum::body::Body;
use http_body_util::BodyExt;
use tower::ServiceExt;

use cynic_kernel::api::rest::{self, AppState, PerIpRateLimiter};
use cynic_kernel::dogs::deterministic::DeterministicDog;
use cynic_kernel::domain::coord::NullCoord;
use cynic_kernel::domain::embedding::NullEmbedding;
use cynic_kernel::domain::storage::NullStorage;
use cynic_kernel::domain::usage::DogUsageTracker;
use cynic_kernel::domain::verdict_cache::VerdictCache;
use cynic_kernel::judge::Judge;

fn test_state(api_key: Option<&str>) -> Arc<AppState> {
    let judge = Arc::new(Judge::new(vec![Box::new(DeterministicDog)]));
    Arc::new(AppState {
        judge,
        storage: Arc::new(NullStorage),
        coord: Arc::new(NullCoord),
        embedding: Arc::new(NullEmbedding),
        usage: Arc::new(tokio::sync::Mutex::new(DogUsageTracker::new())),
        verdict_cache: VerdictCache::new(),
        api_key: api_key.map(|s| s.to_string()),
        rate_limiter: PerIpRateLimiter::new(100),
        judge_limiter: PerIpRateLimiter::new(100),
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

    assert_eq!(resp.status(), 200);
    let v = body_json(resp.into_body()).await;
    // Public: has status, version, phi_max, dog_count — but NOT dogs array
    assert!(v["status"].is_string());
    assert!(v["version"].is_string());
    assert!(v["phi_max"].is_number());
    assert!(v["dog_count"].is_number());
    assert!(v.get("dogs").is_none(), "Public health should not expose dog details");
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

    assert_eq!(resp.status(), 200);
    let v = body_json(resp.into_body()).await;
    // Authenticated: has dogs array, storage status, usage
    assert!(v["dogs"].is_array());
    assert!(v["storage"].is_string());
    assert!(v["total_requests"].is_number());
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
                .body(Body::from(r#"{"content":"The Sicilian Defense is strong","domain":"chess"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let v = body_json(resp.into_body()).await;
    assert!(v["verdict_id"].is_string());
    assert!(v["q_score"]["total"].as_f64().unwrap() > 0.0);
    let verdict = v["verdict"].as_str().unwrap();
    assert!(["Howl", "Wag", "Growl", "Bark"].contains(&verdict));
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

    assert_eq!(resp.status(), 200);
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
