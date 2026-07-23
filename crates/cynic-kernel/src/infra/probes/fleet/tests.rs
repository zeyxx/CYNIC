use super::{FleetProbe, FleetTarget};
use crate::domain::probe::{Probe, ProbeDetails, ProbeStatus};

#[tokio::test]
async fn fleet_probe_no_targets_returns_ok() {
    let probe = FleetProbe::new(vec![]);
    let result = probe.sense().await.expect("should not error");
    assert_eq!(result.status, ProbeStatus::Ok);
}

#[tokio::test]
async fn fleet_probe_unreachable_target() {
    let probe = FleetProbe::new(vec![FleetTarget {
        dog_name: "test-dog".into(),
        health_url: "http://127.0.0.1:19999/health".into(),
        props_url: None,
        models_url: None,
        expected_n_ctx: 0,
        expected_model: String::new(),
        api_key: None,
    }]);
    let result = probe.sense().await.expect("should not error");
    assert_eq!(result.status, ProbeStatus::Unavailable);
    match result.details {
        ProbeDetails::Fleet(ref f) => {
            assert_eq!(f.dogs.len(), 1);
            assert_eq!(f.dogs[0].dog_name, "test-dog");
            assert!(!f.dogs[0].reachable);
        }
        _ => panic!("expected Fleet details"),
    }
}

/// Spins up a minimal Axum server that simulates a llama-server with
/// context drift: /health → 200, /props → n_ctx=2048 (less than expected 8192).
#[tokio::test]
async fn fleet_probe_detects_context_drift() {
    use axum::{Json, Router, routing::get};

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/props",
            get(|| async {
                Json(serde_json::json!({
                    "default_generation_settings": { "n_ctx": 2048 }
                }))
            }),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .inspect_err(|error| {
            tracing::warn!(
                error = %error,
                "skip fleet_probe_detects_context_drift: cannot bind test socket"
            );
        })
        .ok();
    if listener.is_none() {
        return;
    }
    let listener = listener.expect("listener present");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    let base = format!("http://{addr}");
    let probe = FleetProbe::new(vec![FleetTarget {
        dog_name: "drifted-dog".into(),
        health_url: format!("{base}/health"),
        props_url: Some(format!("{base}/props")),
        models_url: None,
        expected_n_ctx: 8192,
        expected_model: String::new(),
        api_key: None,
    }]);

    let result = probe.sense().await.expect("should not error");
    assert_eq!(result.status, ProbeStatus::Ok);

    match result.details {
        ProbeDetails::Fleet(ref f) => {
            assert_eq!(f.dogs.len(), 1);
            let dog = &f.dogs[0];
            assert!(dog.reachable);
            assert_eq!(
                dog.actual_n_ctx,
                Some(2048),
                "should read actual n_ctx from /props"
            );
            assert_eq!(
                dog.expected_n_ctx,
                Some(8192),
                "should carry expected n_ctx"
            );
            // Drift: 2048 < 8192 — logged as error (verified by log output, not assertion)
        }
        _ => panic!("expected Fleet details"),
    }
}

/// When expected_n_ctx is 0 (unconfigured), both fields should be None.
#[tokio::test]
async fn fleet_probe_no_expected_ctx_yields_none() {
    let probe = FleetProbe::new(vec![FleetTarget {
        dog_name: "unconfigured-dog".into(),
        health_url: "http://127.0.0.1:19999/health".into(),
        props_url: None,
        models_url: None,
        expected_n_ctx: 0,
        expected_model: String::new(),
        api_key: None,
    }]);
    let result = probe.sense().await.expect("should not error");
    match result.details {
        ProbeDetails::Fleet(ref f) => {
            assert_eq!(f.dogs[0].expected_n_ctx, None, "0 sentinel → None");
            assert_eq!(f.dogs[0].actual_n_ctx, None, "no props_url → no actual");
        }
        _ => panic!("expected Fleet details"),
    }
}

/// B4: Detects when /v1/models returns a different model than backends.toml declares.
#[tokio::test]
async fn fleet_probe_detects_model_mismatch() {
    use axum::{Json, Router, routing::get};

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/v1/models",
            get(|| async {
                Json(serde_json::json!({
                    "data": [{"id": "gemma-3-4b-it", "object": "model"}]
                }))
            }),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .inspect_err(|error| {
            tracing::warn!(
                error = %error,
                "skip fleet_probe_detects_model_mismatch: cannot bind test socket"
            );
        })
        .ok();
    if listener.is_none() {
        return;
    }
    let listener = listener.expect("listener present");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    let base = format!("http://{addr}");
    let probe = FleetProbe::new(vec![FleetTarget {
        dog_name: "identity-test-dog".into(),
        health_url: format!("{base}/health"),
        props_url: None,
        models_url: Some(format!("{base}/v1/models")),
        expected_n_ctx: 0,
        expected_model: "Mistral-7B-v0.3".into(), // different from what's loaded
        api_key: None,
    }]);

    let result = probe.sense().await.expect("should not error");
    assert_eq!(result.status, ProbeStatus::Ok);

    match result.details {
        ProbeDetails::Fleet(ref f) => {
            let dog = &f.dogs[0];
            assert!(dog.reachable);
            assert_eq!(dog.actual_model.as_deref(), Some("gemma-3-4b-it"));
            assert_eq!(dog.expected_model.as_deref(), Some("Mistral-7B-v0.3"));
            assert!(dog.model_mismatch, "should detect model identity mismatch");
        }
        _ => panic!("expected Fleet details"),
    }
}

/// Model identity match should pass when names contain each other (substring).
#[tokio::test]
async fn fleet_probe_model_match_by_substring() {
    use axum::{Json, Router, routing::get};

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/v1/models",
            get(|| async {
                Json(serde_json::json!({
                    "data": [{"id": "Qwen/Qwen2.5-7B-Instruct", "object": "model"}]
                }))
            }),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .inspect_err(|error| {
            tracing::warn!(
                error = %error,
                "skip fleet_probe_model_match_by_substring: cannot bind test socket"
            );
        })
        .ok();
    if listener.is_none() {
        return;
    }
    let listener = listener.expect("listener present");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    let base = format!("http://{addr}");
    let probe = FleetProbe::new(vec![FleetTarget {
        dog_name: "match-test-dog".into(),
        health_url: format!("{base}/health"),
        props_url: None,
        models_url: Some(format!("{base}/v1/models")),
        expected_n_ctx: 0,
        expected_model: "Qwen2.5-7B".into(), // substring of what's loaded
        api_key: None,
    }]);

    let result = probe.sense().await.expect("should not error");
    match result.details {
        ProbeDetails::Fleet(ref f) => {
            assert!(
                !f.dogs[0].model_mismatch,
                "substring match should not be flagged as mismatch"
            );
        }
        _ => panic!("expected Fleet details"),
    }
}
