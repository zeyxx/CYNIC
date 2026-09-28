//! Cortex startup attention — compact operational pressure surface.
//!
//! This is not a new ledger. It is a read-only projection over kernel health,
//! proprioception, Dog health, and the agent-task mempool.

use axum::{Json, extract::Query, extract::State, http::StatusCode};
use serde::Deserialize;
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};
use crate::domain::storage::AgentTask;

#[derive(Debug, Deserialize)]
pub struct AttentionQuery {
    #[serde(default = "default_task_kind")]
    pub task_kind: String,
    #[serde(default)]
    pub task_domain: Option<String>,
    #[serde(default = "default_task_limit")]
    pub task_limit: u32,
}

fn default_task_kind() -> String {
    "cortex".to_string()
}

fn default_task_limit() -> u32 {
    10
}

/// GET /attention — compact startup brief for cortex sessions.
pub async fn attention_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AttentionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let task_limit = query.task_limit.clamp(1, 25);
    let task_domain = query
        .task_domain
        .as_deref()
        .filter(|value| !value.is_empty());

    let readiness = state.system_health().await;
    let storage_ok = state.storage.ping().await.is_ok();
    let overdue_tasks = if storage_ok {
        state
            .storage
            .list_overdue_agent_tasks(100)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let pending_tasks = if storage_ok {
        state
            .storage
            .list_pending_agent_tasks(&query.task_kind, task_domain, task_limit)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let _sovereign_ids: Vec<String> = vec![];
    let dogs: Vec<serde_json::Value> = vec![];

    let alerts = state
        .introspection_alerts
        .read()
        .map(|alerts| alerts.clone())
        .unwrap_or_else(|poison| {
            tracing::warn!(error = %poison, "introspection_alerts RwLock poisoned");
            Vec::new()
        });
    let environment = state
        .environment
        .read()
        .ok()
        .and_then(|guard| guard.clone())
        .and_then(|snapshot| serde_json::to_value(snapshot).ok());
    let metabolism = crate::domain::observation_counters::snapshot(&state.metrics);

    let readiness_val = serde_json::json!({
        "healthy": readiness.healthy,
        "status": readiness.status,
        "causes": readiness.causes,
    });
    let alerts_val = serde_json::to_value(alerts).unwrap_or_else(|_| serde_json::json!([]));
    let metabolism_val = serde_json::to_value(metabolism).unwrap_or_else(|_| serde_json::json!({}));

    Ok(Json(build_attention_summary(
        &readiness_val,
        &alerts_val,
        &dogs,
        environment.as_ref(),
        &metabolism_val,
        &overdue_tasks,
        &pending_tasks,
        &query.task_kind,
    )))
}

// WHY: Summary builder requires distinct data feeds to compile the operational pressure surface
#[allow(clippy::too_many_arguments)]
fn build_attention_summary(
    readiness: &serde_json::Value,
    alerts: &serde_json::Value,
    dogs: &[serde_json::Value],
    environment: Option<&serde_json::Value>,
    metabolism: &serde_json::Value,
    overdue_tasks: &[AgentTask],
    pending_tasks: &[AgentTask],
    task_kind: &str,
) -> serde_json::Value {
    let critical_alerts: Vec<serde_json::Value> = alerts
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|alert| {
            alert
                .get("severity")
                .and_then(|value| value.as_str())
                .map(|severity| severity == "critical")
                .unwrap_or(false)
        })
        .collect();

    let degraded_dogs: Vec<serde_json::Value> = dogs
        .iter()
        .filter(|&dog| {
            dog.get("circuit")
                .and_then(|value| value.as_str())
                .map(|circuit| circuit != "closed")
                .unwrap_or(false)
                || dog
                    .get("failures")
                    .and_then(|value| value.as_u64())
                    .map(|failures| failures > 0)
                    .unwrap_or(false)
        })
        .cloned()
        .collect();

    let fleet_probe = environment
        .as_ref()
        .and_then(|value| value.get("probes"))
        .and_then(|value| value.as_array())
        .and_then(|probes| {
            probes.iter().find(|probe| {
                probe.get("name").and_then(|value| value.as_str()) == Some("fleet_inventory")
            })
        });
    let fleet_status = fleet_probe
        .and_then(|probe| probe.get("status"))
        .and_then(|value| value.as_str())
        .unwrap_or("missing");
    let fleet_summary = fleet_probe
        .and_then(|probe| probe.get("details"))
        .and_then(|details| details.get("data"))
        .and_then(|data| data.get("summary"));
    let missing_nodes = fleet_summary
        .and_then(|summary| summary.get("declaration_missing_nodes"))
        .and_then(|value| value.as_array())
        .map(|nodes| nodes.iter().take(12).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let provider_evidence_gaps = fleet_summary
        .and_then(|summary| summary.get("provider_evidence_gaps"))
        .and_then(|value| value.as_array())
        .map(|gaps| gaps.iter().take(12).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let missing_node_count = fleet_summary
        .and_then(|summary| summary.get("missing_nodes"))
        .and_then(|value| value.as_u64())
        .unwrap_or(missing_nodes.len() as u64);

    let overdue_task_sample: Vec<serde_json::Value> =
        overdue_tasks.iter().take(10).map(task_projection).collect();
    let pending_task_sample: Vec<serde_json::Value> =
        pending_tasks.iter().take(10).map(task_projection).collect();

    let backlog = metabolism
        .get("backlog")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let digestion_ratio = metabolism
        .get("digestion_ratio")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);

    let attention_level = if !critical_alerts.is_empty()
        || !overdue_task_sample.is_empty()
        || missing_node_count > 0
        || degraded_dogs.iter().any(|dog| {
            dog.get("circuit")
                .and_then(|value| value.as_str())
                .map(|circuit| circuit == "critical")
                .unwrap_or(false)
        }) {
        "critical"
    } else if readiness.get("healthy").and_then(|value| value.as_bool()) != Some(true)
        || backlog > 100
        || digestion_ratio < 38.2
    {
        "degraded"
    } else {
        "normal"
    };

    let mut recommended_actions = Vec::new();
    if missing_node_count > 0 {
        recommended_actions.push(serde_json::json!(
            "Inspect fleet liveness before assuming registry-declared nodes are online"
        ));
    }
    if !degraded_dogs.is_empty() {
        recommended_actions.push(serde_json::json!(
            "Route inference away from degraded dogs or restore their host"
        ));
    }
    if !overdue_task_sample.is_empty() {
        recommended_actions.push(serde_json::json!(
            "Claim or escalate overdue mempool tasks before starting lower-priority work"
        ));
    }
    if backlog > 100 || digestion_ratio < 38.2 {
        recommended_actions.push(serde_json::json!(
            "Prioritize digestion/metabolism work; the organism is accumulating observations faster than it processes them"
        ));
    }

    serde_json::json!({
        "attention_level": attention_level,
        "readiness": readiness,
        "top_causes": {
            "critical_alerts": critical_alerts,
            "overdue_task_count": overdue_task_sample.len(),
            "missing_node_count": missing_node_count,
            "degraded_dog_count": degraded_dogs.len(),
            "backlog": backlog,
            "digestion_ratio": digestion_ratio,
        },
        "fleet": {
            "status": fleet_status,
            "missing_nodes_sample": missing_nodes,
            "provider_evidence_gaps_sample": provider_evidence_gaps,
        },
        "dogs": {
            "degraded": degraded_dogs,
        },
        "mempool": {
            "task_kind": task_kind,
            "overdue_tasks": overdue_task_sample,
            "pending_tasks_sample": pending_task_sample,
        },
        "recommended_actions": recommended_actions,
    })
}

fn task_projection(task: &AgentTask) -> serde_json::Value {
    serde_json::json!({
        "id": task.id,
        "domain": task.domain,
        "priority": task.priority,
        "overdue": task.overdue,
        "due_at": task.due_at,
        "consumer": task.consumer,
        "status": task.status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, domain: &str, overdue: bool) -> AgentTask {
        AgentTask {
            id: id.to_string(),
            kind: "cortex".to_string(),
            domain: domain.to_string(),
            content: "{}".to_string(),
            status: "pending".to_string(),
            result: None,
            created_at: 1_750_720_000_000,
            completed_at: None,
            agent_id: None,
            claimed_by: None,
            error: None,
            due_at: Some("2026-06-24T13:00:00+02:00".to_string()),
            priority: Some(98),
            consumer: Some("cortex".to_string()),
            escalation_policy: None,
            overdue,
            age_seconds: Some(42),
        }
    }

    #[test]
    fn attention_summary_marks_fleet_and_dog_degradation_critical() {
        let readiness = serde_json::json!({
            "healthy": false,
            "status": "degraded",
            "causes": [{"kind": "probes_degraded"}],
        });
        let alerts =
            serde_json::json!([{ "kind": "digestion_starvation", "severity": "critical" }]);
        let dogs = vec![serde_json::json!({
            "id": "qwen36-27b-gpu",
            "circuit": "critical",
            "failures": 3,
            "last_failure_reason": "health_probe",
            "sovereign": true,
        })];
        let environment = serde_json::json!({
            "probes": [{
                "name": "fleet_inventory",
                "status": "degraded",
                "details": {
                    "kind": "fleet_inventory",
                    "data": {
                        "summary": {
                            "missing_nodes": 1,
                            "declaration_missing_nodes": ["cynic-host-gpu"],
                            "provider_evidence_gaps": ["fleet:missing_node:cynic-host-gpu"]
                        }
                    }
                }
            }]
        });
        let metabolism = serde_json::json!({
            "backlog": 1229,
            "digestion_ratio": 0.0,
        });
        let overdue_tasks = vec![task("agent_tasks:offline", "fleet", true)];
        let pending_tasks = vec![task("agent_tasks:fleet", "fleet", false)];

        let summary = build_attention_summary(
            &readiness,
            &alerts,
            &dogs,
            Some(&environment),
            &metabolism,
            &overdue_tasks,
            &pending_tasks,
            "cortex",
        );

        assert_eq!(summary["attention_level"], "critical");
        assert_eq!(summary["top_causes"]["missing_node_count"], 1);
        assert_eq!(summary["top_causes"]["degraded_dog_count"], 1);
        assert_eq!(summary["top_causes"]["overdue_task_count"], 1);
        assert_eq!(
            summary["mempool"]["pending_tasks_sample"][0]["domain"],
            "fleet"
        );
    }
}
