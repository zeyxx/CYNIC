//! GitHub webhook handler — release events for tracked repositories

use axum::{extract::State, http::StatusCode, response::Json};
use serde::Deserialize;
use std::sync::Arc;

use super::super::types::{AppState, ErrorResponse};

/// GitHub release webhook payload (minimal)
#[derive(Debug, Deserialize)]
pub struct GithubReleasePayload {
    /// "published" or "created" when a new release is made
    pub action: Option<String>,
    /// Repository in "owner/repo" format
    #[serde(rename = "repository")]
    pub repo: Option<GithubRepo>,
    /// Release details
    #[serde(rename = "release")]
    pub release: Option<GithubRelease>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GithubRepo {
    #[serde(rename = "full_name")]
    pub full_name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GithubRelease {
    pub tag_name: Option<String>,
    pub name: Option<String>,
    pub published_at: Option<String>,
    #[serde(rename = "html_url")]
    pub html_url: Option<String>,
}

/// Tracked repositories that trigger agent tasks on release
#[derive(Debug)]
pub struct TrackedRepos {
    /// Map of "owner/repo" → target agent kind + domain
    pub repos: std::collections::HashMap<String, (String, String)>,
}

impl Default for TrackedRepos {
    fn default() -> Self {
        Self::new()
    }
}

impl TrackedRepos {
    pub fn new() -> Self {
        let mut repos = std::collections::HashMap::new();
        // nousresearch/hermes-agent → dispatch hermes update task
        repos.insert(
            "NousResearch/hermes-agent".to_string(),
            ("hermes".to_string(), "update".to_string()),
        );
        // zeyxx/CYNIC → dispatch cortex task for kernel rebuild
        repos.insert(
            "zeyxx/CYNIC".to_string(),
            ("cortex".to_string(), "kernel".to_string()),
        );
        Self { repos }
    }

    /// Check if a repo is tracked, return (kind, domain) if so
    pub fn lookup(&self, repo_name: &str) -> Option<&(String, String)> {
        self.repos.get(repo_name)
    }
}

/// POST /webhook/github/release — GitHub release webhook
pub async fn github_release_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GithubReleasePayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Only act on "published" / "created" events (new releases)
    let action = payload.action.as_deref().unwrap_or("published");
    if action != "published" && action != "created" {
        return Ok(Json(serde_json::json!({
            "status": "ignored",
            "reason": "not a release publication"
        })));
    }

    let Some(repo) = payload.repo.clone() else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "missing repository in payload".into(),
            }),
        ));
    };

    let Some(repo_name) = repo.full_name.clone() else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "missing repository.full_name".into(),
            }),
        ));
    };

    let Some(release) = payload.release.clone() else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "missing release in payload".into(),
            }),
        ));
    };

    // Check if this repo is tracked
    let tracked = TrackedRepos::new();
    let Some((kind, domain)) = tracked.lookup(&repo_name) else {
        return Ok(Json(serde_json::json!({
            "status": "ignored",
            "reason": format!("repo {} not tracked", repo_name)
        })));
    };

    // Build task content — structured update notification
    let content = serde_json::json!({
        "source": "github_release",
        "repo": repo_name,
        "tag": release.tag_name,
        "name": release.name,
        "published_at": release.published_at,
        "html_url": release.html_url,
        "action": "update",
    });

    let task_id = format!(
        "webhook:{}:{}",
        repo_name.replace('/', "_"),
        crate::infra::crypto::generate_secure_id()
    );

    let agent_task = crate::domain::storage::AgentTask {
        id: task_id.clone(),
        kind: kind.clone(),
        domain: domain.clone(),
        content: content.to_string(),
        status: "pending".to_string(),
        result: None,
        created_at: chrono::Utc::now().timestamp_millis(),
        completed_at: None,
        agent_id: Some("webhook:github".to_string()),
        claimed_by: None,
        error: None,
        due_at: None,
        priority: Some(10), // High priority for updates
        consumer: Some(kind.clone()),
        escalation_policy: None,
        overdue: false,
        age_seconds: None,
    };

    let task_id = state
        .storage
        .store_agent_task(&agent_task)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, repo = %repo_name, "webhook task storage failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Storage unreachable: {e}"),
                }),
            )
        })?;

    let _ = state
        .event_tx
        .send(crate::domain::events::KernelEvent::AgentTaskCreated {
            task_id: task_id.clone(),
            kind: kind.clone(),
            domain: domain.clone(),
        });

    tracing::info!(
        task_id = %task_id,
        repo = %repo_name,
        tag = %release.tag_name.as_deref().unwrap_or(""),
        kind = %kind,
        domain = %domain,
        "GitHub release webhook processed"
    );

    Ok(Json(serde_json::json!({
        "status": "dispatched",
        "task_id": task_id,
        "repo": repo_name,
        "kind": kind,
        "domain": domain,
    })))
}
