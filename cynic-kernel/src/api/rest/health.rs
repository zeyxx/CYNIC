//! REST API handlers for liveness, readiness, health, agents and metrics — read-only observability.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use super::response::coordination_error;
use super::types::{AppState, DogHealthResponse, ErrorResponse};
use crate::domain::dog::{AXIOM_NAMES, PHI_INV};
use crate::domain::health_gate::count_healthy_dogs;

/// GET /live — Liveness probe. Returns 200 if the process is running.
/// No dependencies checked — this is "is the kernel alive?" for systemd/k8s.
pub async fn liveness_handler() -> StatusCode {
    StatusCode::OK
}

/// GET /ready — Readiness probe. Returns 200 if the kernel can serve requests.
/// F22: Caches DB ping result (30s TTL) to avoid hammering storage on every probe.
/// Dog health is O(1) (reads circuit breaker state) — no caching needed.
pub async fn readiness_handler(State(state): State<Arc<AppState>>) -> StatusCode {
    let assessment = state.system_health().await;
    if assessment.healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

pub async fn health_handler(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> (StatusCode, Json<serde_json::Value>) {
    // Check if caller has valid auth — return full details only if authenticated.
    // Uses constant_time_eq to prevent timing oracle (same as auth_middleware).
    let authenticated = match &state.api_key {
        Some(key) => request
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .is_some_and(|t| super::middleware::constant_time_eq(t.as_bytes(), key.as_bytes())),
        None => true, // No auth configured → everyone gets full details
    };

    let readiness = state.system_health().await;
    let http_code = if readiness.healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    // Public: minimal info only — no version, no dog count, no topology.
    // HTTP status code tells the story: 200 = healthy, 503 = degraded/critical.
    // Any monitoring tool can check without parsing JSON: curl -sf URL || alert
    // KC3: version removed — leaks git SHA + commit count to unauthenticated callers.
    if !authenticated {
        return (
            http_code,
            Json(serde_json::json!({
                "status": readiness.status,
                "phi_max": PHI_INV,
            })),
        );
    }

    // Authenticated: full details
    let judge = state.judge.load_full();
    let dog_health = judge.dog_health();
    let (healthy_dogs, total_dogs) = count_healthy_dogs(&dog_health);
    let live_dog_ids = judge.dog_ids();
    let contract_delta = {
        let guard = state
            .system_contract
            .read()
            .unwrap_or_else(|e| e.into_inner());
        guard.assess(&live_dog_ids)
    };
    let storage_ok = state.storage.ping().await.is_ok();
    let probes_degraded =
        crate::domain::probe::EnvironmentSnapshot::is_degraded(&state.environment);
    let stale_tasks = state.task_health.readiness_stale_tasks();

    let dog_health_detail = judge.dog_health_detailed();
    let dogs: Vec<DogHealthResponse> = dog_health_detail
        .into_iter()
        .map(|(id, circuit, failures, reason, open_secs)| {
            let kind = if id == "deterministic-dog" {
                "heuristic"
            } else {
                "inference"
            }
            .to_string();
            DogHealthResponse {
                id,
                kind,
                circuit,
                failures,
                last_failure_reason: reason.map(|r| r.as_str().to_string()),
                open_since_secs: open_secs,
            }
        })
        .collect();

    let organ_quality: Vec<serde_json::Value> = judge
        .dog_quality_snapshot()
        .into_iter()
        .map(|(id, stats)| {
            let mut obj = serde_json::json!({
                "dog": id,
                "json_valid_rate": stats.json_valid_rate(),
                "capability_limit_rate": stats.capability_limit_rate(),
                "total_calls": stats.total_calls,
                "success_count": stats.success_count,
                "mean_latency_ms": (stats.mean_latency_ms() * 10.0).round() / 10.0,
                "failures": {
                    "zero_flood": stats.zero_flood_count,
                    "collapse": stats.collapse_count,
                    "parse_error": stats.parse_error_count,
                    "timeout": stats.timeout_count,
                    "api_error": stats.api_error_count,
                },
            });
            if let Some(ts) = &stats.last_success {
                obj["last_success"] = serde_json::Value::String(ts.clone());
            }
            if stats.success_count > 0 {
                let tok_per_sec = if stats.mean_latency_ms() > 0.0 {
                    (stats.total_completion_tokens as f64 / stats.success_count as f64)
                        / (stats.mean_latency_ms() / 1000.0)
                } else {
                    0.0
                };
                obj["tok_per_sec"] = serde_json::json!((tok_per_sec * 10.0).round() / 10.0);
            }
            obj
        })
        .collect();

    let (total_requests, total_tokens, estimated_cost_usd, uptime_seconds) = {
        let usage = state.usage.lock().await;
        (
            usage.all_time_requests(),
            usage.total_tokens(),
            usage.estimated_cost_usd(),
            usage.uptime_seconds(),
        )
    };

    // Onchain submission observability (K15: producer-consumer audit)
    let (verdicts_queued, verdicts_submitted, verdicts_confirmed, verdicts_failed) = state
        .storage
        .queue_status_counts()
        .await
        .unwrap_or_default(); // K14: degraded if storage unavailable

    // Proprioception: crystal state summary (best-effort, non-blocking)
    let crystal_summary = match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        state.storage.list_crystals(200),
    )
    .await
    {
        Ok(Ok(crystals)) => {
            use crate::domain::ccm::CrystalState;
            let (mut forming, mut crystallized, mut canonical, mut decaying) =
                (0u32, 0u32, 0u32, 0u32);
            for c in &crystals {
                match c.state {
                    CrystalState::Forming => forming += 1,
                    CrystalState::Crystallized => crystallized += 1,
                    CrystalState::Canonical => canonical += 1,
                    CrystalState::Decaying => decaying += 1,
                    CrystalState::Dissolved => {}
                }
            }
            serde_json::json!({
                "total": crystals.len(),
                "forming": forming,
                "crystallized": crystallized,
                "canonical": canonical,
                "decaying": decaying,
                "ever_crystallized": crystallized + canonical > 0,
            })
        }
        _ => serde_json::json!({ "error": "unavailable" }),
    };

    let (expected_dogs, expected_count) = {
        let guard = state
            .system_contract
            .read()
            .unwrap_or_else(|e| e.into_inner());
        (guard.expected_dogs().to_vec(), guard.expected_count())
    };

    // Senses: organism perceiving its own external data stores (K15 consumer)
    let mut senses_report: Vec<serde_json::Value> = Vec::new();
    for sense in &state.senses {
        let health = sense.health().await;
        let (health_str, reason) = match &health {
            crate::domain::organ::OrganHealth::Alive => ("alive", None),
            crate::domain::organ::OrganHealth::Degraded { reason } => {
                ("degraded", Some(reason.as_str()))
            }
            crate::domain::organ::OrganHealth::Dead { reason } => ("dead", Some(reason.as_str())),
        };
        let freshness_secs = sense
            .freshness()
            .await
            .map(|d| d.as_secs())
            .unwrap_or(u64::MAX);
        let mut entry = serde_json::json!({
            "name": sense.name(),
            "health": health_str,
            "freshness_secs": if freshness_secs == u64::MAX { serde_json::Value::Null } else { serde_json::json!(freshness_secs) },
        });
        if let Some(r) = reason {
            entry["reason"] = serde_json::json!(r);
        }
        senses_report.push(entry);
    }

    (
        http_code,
        Json(serde_json::json!({
            "status": readiness.status,
            "version": env!("CYNIC_VERSION"),
            "phi_max": PHI_INV,
            "axioms": AXIOM_NAMES,
            "dogs": dogs,
            "storage": if storage_ok { "connected" } else { "down" },
            "storage_namespace": state.storage_info.namespace,
            "storage_database": state.storage_info.database,
            "storage_metrics": state.storage_metrics(),
            "embedding": if tokio::time::timeout(std::time::Duration::from_secs(2), state.embedding.embed("h")).await.map(|r| r.is_ok()).unwrap_or(false) { "sovereign" } else { "unavailable" },
            "crystals": crystal_summary,
            "organ_quality": organ_quality,
            "contract": {
                "expected_dogs": expected_dogs,
                "expected_count": expected_count,
                "missing_dogs": &contract_delta.missing,
                "unexpected_dogs": &contract_delta.unexpected,
                "fulfilled": contract_delta.fulfilled,
            },
            "readiness": {
                "status": readiness.status,
                "healthy": readiness.healthy,
                "healthy_dogs": healthy_dogs,
                "total_dogs": total_dogs,
                "storage_ok": storage_ok,
                "probes_degraded": probes_degraded,
                "stale_tasks": stale_tasks,
                "causes": readiness.causes,
            },
            "environment": state.environment.read().ok().and_then(|e| e.clone()),
            "chain_verified": state.chain_verified.load(std::sync::atomic::Ordering::Relaxed),
            "verdict_cache_size": state.verdict_cache.len(),
            "background_tasks": state.task_health.snapshot(),
            "total_requests": total_requests,
            "total_tokens": total_tokens,
            "estimated_cost_usd": estimated_cost_usd,
            "uptime_seconds": uptime_seconds,
            "alerts": state.introspection_alerts.read()
                .map(|a| a.clone())
                .unwrap_or_else(|e| {
                    tracing::warn!(error = %e, "introspection_alerts RwLock poisoned");
                    Vec::new()
                }),
            "onchain_observability": {
                "verdicts_queued": verdicts_queued,
                "verdicts_submitted": verdicts_submitted,
                "verdicts_confirmed": verdicts_confirmed,
                "verdicts_failed": verdicts_failed,
            },
            "senses": senses_report,
        })),
    )
}

/// GET /agents — show active agent sessions and their claims (requires auth)
pub async fn agents_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.coord.who(None).await {
        Ok(snapshot) => Ok(Json(
            serde_json::to_value(snapshot.into_summary()).unwrap_or_default(),
        )),
        Err(e) => {
            tracing::warn!(error = %e, "agents query failed");
            Err(coordination_error())
        }
    }
}

/// GET /metrics — Prometheus text exposition format.
/// Auth required (KC3) — leaks Dog roster, failure modes, circuit states, token counts.
pub async fn metrics_handler(
    State(state): State<Arc<AppState>>,
) -> (
    StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    String,
) {
    let mut out = state.metrics.render_prometheus();

    // Verdict cache size (gauge)
    {
        use std::fmt::Write;
        let _ = writeln!(
            out,
            "# HELP cynic_verdict_cache_size Current verdict cache entries"
        );
        let _ = writeln!(out, "# TYPE cynic_verdict_cache_size gauge");
        let _ = writeln!(
            out,
            "cynic_verdict_cache_size {}",
            state.verdict_cache.len()
        );
    }

    // Per-dog metrics from usage tracker
    {
        let usage = state.usage.lock().await;
        let merged = usage.merged_dogs();
        let mut dog_data: Vec<(String, u64, u64, u64, u64)> = merged
            .into_iter()
            .map(|(id, u)| {
                (
                    id,
                    u.requests,
                    u.failures,
                    u.total_latency_ms,
                    u.total_tokens(),
                )
            })
            .collect();
        dog_data.sort_by(|a, b| a.0.cmp(&b.0));

        let judge = state.judge.load_full();
        let circuit_states = judge.dog_health();
        crate::domain::metrics::append_dog_metrics(&mut out, &dog_data, &circuit_states);
    }

    // Organ quality metrics
    {
        let judge = state.judge.load_full();
        let snapshots = judge.dog_quality_snapshot();
        crate::domain::metrics::append_organ_metrics(&mut out, &snapshots);
    }

    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        out,
    )
}

/// GET /state-history?since=RFC3339&limit=N — Hash-chained organism state log.
/// Auth required. Returns state blocks ordered by seq ASC.
pub async fn state_history_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let since = params
        .get("since")
        .map(|s| s.as_str())
        .unwrap_or("1970-01-01T00:00:00Z");
    let limit: u32 = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    match state.storage.list_state_blocks(since, limit).await {
        Ok(blocks) => {
            let chain_valid = blocks.windows(2).all(|w| w[1].prev_hash == w[0].hash);
            let blocks_valid = blocks.iter().all(|b| b.verify());
            Ok(Json(serde_json::json!({
                "blocks": blocks,
                "count": blocks.len(),
                "chain_valid": chain_valid,
                "blocks_valid": blocks_valid,
            })))
        }
        Err(e) => {
            tracing::warn!(error = %e, "state-history query failed");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("state-history query failed: {e}"),
                }),
            ))
        }
    }
}

// Logic tests live in domain::health_gate::tests — single source of truth.
// This handler only maps (status, is_healthy) → HTTP status code.
