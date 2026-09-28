use crate::domain::dog::{AXIOM_NAMES, PHI_INV};
/// REST API handlers for liveness, readiness, health, agents and metrics — read-only observability.
use axum::{extract::State, http::StatusCode, response::Json};
use std::sync::Arc;
use std::time::Duration;

use super::response::coordination_error;
use super::types::{AppState, DogHealthResponse, ErrorResponse};

use crate::domain::health_gate::count_healthy_dogs;
use crate::domain::storage::{AgentTask, Observation};

const HEALTH_STORAGE_TIMEOUT: Duration = Duration::from_secs(3);
const HEALTH_SENSE_TIMEOUT: Duration = Duration::from_secs(2);

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

/// GET /health — Full system topology and diagnostics.
/// Auth required (T1, KC3) — leaks Dog roster, circuit states, version, token counts.
/// Unauthenticated callers get 401 from middleware (never reach this handler).
/// For unauthenticated probes, use /live (200) or /ready (200/503).
// WHY: Health response contains many optional legacy flags, restructuring breaks the API.
#[allow(clippy::type_complexity)]
pub async fn health_handler(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let readiness = state.system_health().await;
    let http_code = if readiness.healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    // Authenticated: full details

    let mut dog_health: Vec<(String, String, u32)> =
        vec![("deterministic-dog".to_string(), "Closed".to_string(), 0)];
    for cfg in &state.backend_configs {
        dog_health.push((cfg.name.clone(), "Closed".to_string(), 0));
    }
    let (healthy_dogs, total_dogs) = count_healthy_dogs(&dog_health);
    let mut live_dog_ids = vec!["deterministic-dog".to_string()];
    for cfg in &state.backend_configs {
        live_dog_ids.push(cfg.name.clone());
    }
    let contract_delta = {
        let guard = state
            .system_contract
            .read()
            .unwrap_or_else(|e| e.into_inner());
        guard.assess(&live_dog_ids)
    };
    let storage_ok = tokio::time::timeout(HEALTH_STORAGE_TIMEOUT, state.storage.ping())
        .await
        .ok()
        .and_then(Result::ok)
        .is_some();
    let probes_degraded =
        crate::domain::probe::EnvironmentSnapshot::is_degraded(&state.environment);
    let stale_tasks = state.task_health.readiness_stale_tasks();
    let overdue_tasks = if storage_ok {
        tokio::time::timeout(
            HEALTH_STORAGE_TIMEOUT,
            state.storage.list_overdue_agent_tasks(100),
        )
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default()
    } else {
        Vec::new()
    };
    emit_overdue_task_observations(&state, &overdue_tasks).await;

    let mut dog_health_detail = vec![(
        "deterministic-dog".to_string(),
        "Closed".to_string(),
        0,
        None,
        None,
    )];
    for cfg in &state.backend_configs {
        dog_health_detail.push((cfg.name.clone(), "Closed".to_string(), 0, None, None));
    }
    let sovereign_ids: Vec<String> = state
        .backend_configs
        .iter()
        .filter(|cfg| cfg.sovereign)
        .map(|cfg| cfg.name.clone())
        .collect();
    let dogs: Vec<DogHealthResponse> = dog_health_detail
        .into_iter()
        .map(|(id, circuit, failures, reason, open_secs)| {
            let kind = if id == "deterministic-dog" {
                "heuristic"
            } else {
                "inference"
            }
            .to_string();
            let sovereign = sovereign_ids.iter().any(|s| s == &id);
            DogHealthResponse {
                id,
                kind,
                sovereign,
                circuit,
                failures,
                last_failure_reason: reason,
                open_since_secs: open_secs,
            }
        })
        .collect();

    let organ_quality: Vec<serde_json::Value> = vec![];

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
    let (verdicts_queued, verdicts_submitted, verdicts_confirmed, verdicts_failed) =
        tokio::time::timeout(HEALTH_STORAGE_TIMEOUT, state.storage.queue_status_counts())
            .await
            .ok()
            .and_then(Result::ok)
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
        let health = tokio::time::timeout(HEALTH_SENSE_TIMEOUT, sense.health()).await;
        let (health_str, reason) = match &health {
            Ok(crate::domain::observation::ObservationHealth::Alive) => ("alive", None),
            Ok(crate::domain::observation::ObservationHealth::Degraded { reason }) => {
                ("degraded", Some(reason.as_str()))
            }
            Ok(crate::domain::observation::ObservationHealth::Dead { reason }) => {
                ("dead", Some(reason.as_str()))
            }
            Err(_) => ("degraded", Some("health_timeout")),
        };
        let freshness_secs = tokio::time::timeout(HEALTH_SENSE_TIMEOUT, sense.freshness())
            .await
            .ok()
            .and_then(Result::ok)
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

    // Push-producer heartbeat report — absence detection (K15).
    // Only show agents with count >= 2 (filters one-shot POSTs).
    let producer_report: Vec<serde_json::Value> = {
        let map = state
            .producer_heartbeats
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let mut report: Vec<_> = map
            .iter()
            .filter(|(_, hb)| hb.count >= 2)
            .map(|(agent_id, hb)| {
                let age_secs = hb.last_seen.elapsed().as_secs();
                let is_permanent = crate::introspection::is_permanent_source(agent_id);
                let health = if !is_permanent {
                    "ephemeral"
                } else if age_secs > 600 {
                    "silent"
                } else {
                    "alive"
                };
                serde_json::json!({
                    "agent_id": agent_id,
                    "last_seen_secs": age_secs,
                    "observations": hb.count,
                    "health": health,
                })
            })
            .collect();
        report.sort_by(|a, b| {
            a["agent_id"]
                .as_str()
                .unwrap_or("")
                .cmp(b["agent_id"].as_str().unwrap_or(""))
        });
        report
    };

    (
        http_code,
        Json(serde_json::json!({
            "status": readiness.status,
            "version": env!("CARGO_PKG_VERSION"),
            "build": {
                "describe": env!("CYNIC_VERSION"),
                "git_sha": env!("CYNIC_GIT_SHA"),
                "profile": env!("CYNIC_BUILD_PROFILE"),
            },
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
                "overdue_agent_tasks": overdue_tasks,
                "causes": readiness.causes,
            },
            "environment": state.environment.read().ok().and_then(|e| e.clone()),
            "epoche": {
                "window_size": 0,
                "threshold": 0,
            },
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
            // K15: Seam 3 — Pipeline pressure signal for Soma emergence
            // Soma (resource orchestrator) reads this to auto-scale compute allocation
            "pipeline_pressure": {
                "pending_verdicts": verdicts_queued,
                "failed_verdicts": verdicts_failed,
                "success_rate": if verdicts_submitted > 0 {
                    ((verdicts_submitted - verdicts_failed) as f64 / verdicts_submitted as f64 * 100.0).round() / 100.0
                } else {
                    100.0
                },
                "bottleneck": match (storage_ok, healthy_dogs > 0, probes_degraded) {
                    (false, _, _) => "storage",
                    (true, false, _) => "no_healthy_dogs",
                    (true, true, true) => "environment_degraded",
                    _ => "healthy",
                },
                "pressure_level": match verdicts_queued {
                    0..=5 => "low",
                    6..=20 => "moderate",
                    21..=50 => "high",
                    _ => "critical",
                },
            },
            "slot_utilization": state.slot_tracker.health_summary(),
            // The key stays "metabolism": it is a published REST field read by dashboards.
            // What it carries is now ObservationCounters, which is what it has always
            // measured — the observation funnel, not the host's resource load.
            "metabolism": crate::domain::observation_counters::snapshot(&state.metrics),
            "senses": senses_report,
            "producers": producer_report,
        })),
    )
}

async fn emit_overdue_task_observations(state: &Arc<AppState>, overdue_tasks: &[AgentTask]) {
    let to_emit: Vec<AgentTask> = {
        let Ok(alerted) = state.overdue_task_alerted.read() else {
            return;
        };
        overdue_tasks
            .iter()
            .filter(|task| !alerted.contains_key(&task.id))
            .cloned()
            .collect()
    };

    for task in to_emit {
        let now = chrono::Utc::now().timestamp_millis();
        let observation = Observation {
            project: "CYNIC".to_string(),
            agent_id: "cynic-kernel".to_string(),
            tool: "agent_task_overdue".to_string(),
            target: task.id.clone(),
            domain: task.domain.clone(),
            status: "critical".to_string(),
            context: format!("agent task overdue: {}", task.id),
            session_id: "kernel-overdue-monitor".to_string(),
            timestamp: now,
            tags: vec![
                "agent-task".to_string(),
                "overdue".to_string(),
                "anti-forgetting".to_string(),
            ],
            source_tier: "permanent".to_string(),
            value: Some(serde_json::json!({
                "task_id": task.id.clone(),
                "kind": task.kind.clone(),
                "domain": task.domain.clone(),
                "due_at": task.due_at.clone(),
                "priority": task.priority,
                "consumer": task.consumer.clone(),
                "escalation_policy": task.escalation_policy.clone(),
                "age_seconds": task.age_seconds,
            })),
            confidence: Some("observed".to_string()),
            consumer: task
                .consumer
                .clone()
                .or_else(|| Some("human+cortex".to_string())),
            action: Some(
                task.escalation_policy
                    .clone()
                    .unwrap_or_else(|| "review pending overdue agent task".to_string()),
            ),
            depends_on: Vec::new(),
            maturity: Some(1.0),
            hash: String::new(),
            prev_hash: String::new(),
            observers: vec!["cynic-kernel".to_string()],
            consensus_score: Some(1.0),
        };

        if state.storage.store_observation(&observation).await.is_ok()
            && let Ok(mut alerted) = state.overdue_task_alerted.write()
        {
            alerted.insert(observation.target.clone(), now.to_string());
        }
    }
}

/// GET /coord/agents — show active agent sessions, their claims, and connected websocket nodes
pub async fn agents_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.coord.who(None).await {
        Ok(snapshot) => {
            let mut val = serde_json::to_value(snapshot.into_summary()).unwrap_or_default();
            let nodes = state.node_registry.list_active().await;
            if let Some(obj) = val.as_object_mut() {
                obj.insert("websocket_nodes".to_string(), serde_json::json!(nodes));
            }
            Ok(Json(val))
        }
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
        let _merged = usage.merged_dogs();
        let dog_data: Vec<(String, u64, u64, u64, u64)> = vec![];
        let circuit_states: Vec<(String, String, u32)> = vec![];
        crate::domain::metrics::append_dog_metrics(&mut out, &dog_data, &circuit_states);
    }

    // Organ quality metrics
    {
        let _snapshots: std::collections::HashMap<String, serde_json::Value> =
            std::collections::HashMap::new();
        // crate::domain::metrics::append_organ_metrics(&mut out, &snapshots);
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
