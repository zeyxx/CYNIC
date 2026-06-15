//! REST handlers for coordination — POST /coord/register, /coord/claim, /coord/release.
//! These expose the same CoordPort methods as MCP, enabling hooks and external
//! clients to coordinate without MCP stdio.

use axum::{extract::State, http::StatusCode, response::Json};
use serde::Deserialize;
use std::sync::Arc;

use super::response::coordination_error;
use super::types::{AppState, ErrorResponse};
use crate::domain::coord::ClaimResult;

fn validate_agent_id(id: &str) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    crate::domain::coord::validate_agent_id(id).map_err(|msg| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: msg.into() }),
        )
    })
}

// ── Request types ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub agent_id: String,
    pub intent: String,
    pub agent_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClaimRequest {
    pub agent_id: String,
    pub target: String,
    pub claim_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BatchClaimRequest {
    pub agent_id: String,
    pub targets: Vec<String>,
    pub claim_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseRequest {
    pub agent_id: String,
    pub target: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CoordHeartbeatRequest {
    pub agent_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ScopeRequest {
    pub agent_id: String,
    pub scope: String,
}

// ── Handlers ──────────────────────────────────────────────

pub async fn coord_register_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    validate_agent_id(&req.agent_id)?;
    if req.intent.is_empty() || req.intent.chars().count() > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "intent must be 1-500 characters".into(),
            }),
        ));
    }
    let agent_type = req.agent_type.unwrap_or_else(|| "unknown".into());
    if agent_type.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "agent_type must be under 64 characters".into(),
            }),
        ));
    }

    state
        .coord
        .register_agent(&req.agent_id, &agent_type, &req.intent)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "coord register failed");
            coordination_error()
        })?;

    // Heartbeat: keep session alive (REST has no implicit heartbeat like MCP)
    let _ = state.coord.heartbeat(&req.agent_id).await; // ok: fire-and-forget

    let _ = state
        .coord
        .store_audit(
            // ok: fire-and-forget
            "cynic_coord_register",
            &req.agent_id,
            &serde_json::json!({"intent": req.intent, "agent_type": agent_type, "source": "rest"})
                .to_string(),
        )
        .await;

    let _ = state
        .event_tx
        .send(crate::domain::events::KernelEvent::SessionRegistered {
            agent_id: req.agent_id.clone(),
        }); // ok: no subscribers = silent

    Ok(Json(serde_json::json!({
        "status": "registered",
        "agent_id": req.agent_id,
    })))
}

pub async fn coord_claim_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ClaimRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    validate_agent_id(&req.agent_id)?;
    if req.target.is_empty() || req.target.len() > 256 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "target must be 1-256 characters".into(),
            }),
        ));
    }
    let claim_type = req.claim_type.unwrap_or_else(|| "file".into());
    if claim_type.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "claim_type must be under 64 characters".into(),
            }),
        ));
    }

    // K15 Gate: Enforce compliance threshold before allowing claims
    let observations = state
        .storage
        .list_observations_raw(None, Some(&req.agent_id), 500)
        .await
        .unwrap_or_default();

    let comp =
        crate::domain::compliance::score_session(&req.agent_id, &req.agent_id, &observations);
    if comp.score < crate::domain::dog::PHI_INV3 {
        tracing::error!(agent_id = %req.agent_id, score = %comp.score, "compliance: claim rejected");
        return Err((
            StatusCode::PRECONDITION_FAILED,
            Json(ErrorResponse {
                error: format!(
                    "CRITICAL NON-COMPLIANCE (score={:.3}). Claim REJECTED. Warnings: {}",
                    comp.score,
                    comp.warnings.join("; ")
                ),
            }),
        ));
    }

    match state
        .coord
        .claim(&req.agent_id, &req.target, &claim_type)
        .await
    {
        Ok(ClaimResult::Claimed) => {
            let _ = state.coord.heartbeat(&req.agent_id).await; // ok: fire-and-forget
            let _ = state.coord.store_audit( // ok: fire-and-forget
                "cynic_coord_claim", &req.agent_id,
                &serde_json::json!({"target": req.target, "claim_type": claim_type, "source": "rest"}).to_string(),
            ).await;
            Ok(Json(serde_json::json!({
                "status": "claimed",
                "agent_id": req.agent_id,
                "target": req.target,
            })))
        }
        // Agent Protocol v1 (T1): claim as signal, not lock.
        // Claim succeeds, conflicts reported. The cost of ignoring
        // (wasted tokens at PROPOSE) is the natural incentive.
        Ok(ClaimResult::Conflict(conflicts)) => {
            let conflict_list: Vec<_> = conflicts
                .iter()
                .map(|c| serde_json::json!({"agent_id": &c.agent_id, "claimed_at": &c.claimed_at}))
                .collect();
            let _ = state.coord.store_audit(
                "cynic_coord_claim", &req.agent_id,
                &serde_json::json!({"target": req.target, "claim_type": claim_type, "conflicts": &conflict_list, "source": "rest"}).to_string(),
            ).await;
            Ok(Json(serde_json::json!({
                "status": "claimed_with_conflicts",
                "agent_id": req.agent_id,
                "target": req.target,
                "conflicts": conflict_list,
            })))
        }
        Err(e) => {
            tracing::warn!(error = %e, "coord claim failed");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "coordination unavailable".into(),
                }),
            ))
        }
    }
}

pub async fn coord_claim_batch_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BatchClaimRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    validate_agent_id(&req.agent_id)?;
    if req.targets.is_empty() || req.targets.len() > 20 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "targets must be 1-20 items".into(),
            }),
        ));
    }
    for t in &req.targets {
        if t.is_empty() || t.len() > 256 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!(
                        "target '{}' must be 1-256 characters",
                        t.chars().take(32).collect::<String>()
                    ),
                }),
            ));
        }
    }

    let claim_type = req.claim_type.unwrap_or_else(|| "file".into());
    if claim_type.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "claim_type must be under 64 characters".into(),
            }),
        ));
    }

    let result = state
        .coord
        .claim_batch(&req.agent_id, &req.targets, &claim_type)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "coord claim-batch failed");
            coordination_error()
        })?;

    let _ = state.coord.store_audit( // ok: fire-and-forget
        "cynic_coord_claim_batch", &req.agent_id,
        &serde_json::json!({"targets": req.targets, "claimed": result.claimed.len(), "conflicts": result.conflicts.len(), "source": "rest"}).to_string(),
    ).await;

    let conflicts: Vec<serde_json::Value> = result
        .conflicts
        .iter()
        .map(|(target, infos)| {
            serde_json::json!({
                "target": target,
                "held_by": infos.iter().map(|c| &c.agent_id).collect::<Vec<_>>(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "agent_id": req.agent_id,
        "claimed": result.claimed,
        "conflicts": conflicts,
    })))
}

pub async fn coord_release_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ReleaseRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    validate_agent_id(&req.agent_id)?;
    if let Some(ref t) = req.target
        && (t.is_empty() || t.len() > 256)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "target must be 1-256 characters".into(),
            }),
        ));
    }

    let desc = state
        .coord
        .release(&req.agent_id, req.target.as_deref())
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "coord release failed");
            coordination_error()
        })?;

    let _ = state
        .coord
        .store_audit(
            // ok: fire-and-forget
            "cynic_coord_release",
            &req.agent_id,
            &serde_json::json!({"target": req.target, "source": "rest"}).to_string(),
        )
        .await;

    Ok(Json(serde_json::json!({
        "status": "released",
        "agent_id": req.agent_id,
        "detail": desc,
    })))
}

pub async fn coord_heartbeat_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CoordHeartbeatRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    validate_agent_id(&req.agent_id)?;

    state.coord.heartbeat(&req.agent_id).await.map_err(|e| {
        tracing::warn!(error = %e, "coord heartbeat failed");
        coordination_error()
    })?;

    Ok(Json(serde_json::json!({
        "status": "heartbeat_accepted",
        "agent_id": req.agent_id,
    })))
}

pub async fn coord_scope_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ScopeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    validate_agent_id(&req.agent_id)?;
    if req.scope.is_empty() || req.scope.chars().count() > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "scope must be 1-500 characters".into(),
            }),
        ));
    }

    state
        .coord
        .scope_update(&req.agent_id, &req.scope)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "coord scope update failed");
            coordination_error()
        })?;

    Ok(Json(serde_json::json!({
        "status": "scope_updated",
        "agent_id": req.agent_id,
    })))
}

/// GET /coord/who — show all active agents and their claims.
/// Optional query param: ?agent_id=xxx to filter by agent.
pub async fn coord_who_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::BTreeMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let agent_filter = params.get("agent_id").map(|s| s.as_str());

    let snapshot = state.coord.who(agent_filter).await.map_err(|e| {
        tracing::warn!(error = %e, "coord who failed");
        coordination_error()
    })?;

    Ok(Json(serde_json::json!({
        "agents": snapshot.agents,
        "claims": snapshot.claims,
    })))
}

/// GET /coord/topology — exposes the current fleet topology from infra/registry.json.
pub async fn coord_topology_handler()
-> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let project_root = std::env::var("CYNIC_PROJECT_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::process::Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| std::path::PathBuf::from(s.trim()))
                .unwrap_or_else(|| {
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                })
        });

    let registry_path = project_root.join("infra/registry.json");
    let content = std::fs::read_to_string(&registry_path).map_err(|e| {
        tracing::warn!(error = %e, path = %registry_path.display(), "failed to read registry.json");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "topology unavailable".into(),
            }),
        )
    })?;

    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        tracing::warn!(error = %e, "failed to parse registry.json");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "topology malformed".into(),
            }),
        )
    })?;

    Ok(Json(json))
}

/// POST /coord/upgrade — receives a new compiled cynic-kernel binary.
/// Replaces the current executable and schedules a systemd restart.
/// Handled natively without external scripts or SSH.
pub async fn coord_upgrade_handler(
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if body.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Empty binary payload".into(),
            }),
        ));
    }

    let bin_dir = std::env::var("CYNIC_BIN_DIR").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{home}/bin")
    });

    let target_path = format!("{bin_dir}/cynic-kernel");
    let tmp_path = format!("{bin_dir}/cynic-kernel.new");

    // 1. Write the payload to a temporary file
    if let Err(e) = std::fs::write(&tmp_path, &body) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to write tmp binary: {e}"),
            }),
        ));
    }

    // 2. Backup the current binary for rollback (ignore if it doesn't exist)
    let prev_path = format!("{bin_dir}/cynic-kernel.prev");
    let _ = std::fs::copy(&target_path, &prev_path);

    // 3. Make it executable
    use std::os::unix::fs::PermissionsExt;
    if let Ok(mut perms) = std::fs::metadata(&tmp_path).map(|m| m.permissions()) {
        perms.set_mode(0o755);
        let _ = std::fs::set_permissions(&tmp_path, perms);
    }

    // 4. Atomically rename tmp_path -> target_path
    if let Err(e) = std::fs::rename(&tmp_path, &target_path) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to swap binary: {e}"),
            }),
        ));
    }

    // 5. Spawn background task to trigger restart
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let _ = tokio::process::Command::new("systemctl")
            .arg("--user")
            .arg("restart")
            .arg("cynic-kernel")
            .spawn();
    });

    Ok(Json(serde_json::json!({
        "status": "upgrade_scheduled",
        "bytes_received": body.len(),
        "restarting_in": "500ms"
    })))
}

/// D4: Sovereign Auto-Rollback
/// Replaces the current `cynic-kernel` with `cynic-kernel.prev` and triggers `systemctl --user restart`.
pub async fn coord_rollback_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let bin_dir = std::env::var("CYNIC_BIN_DIR").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{home}/bin")
    });

    let target_path = format!("{bin_dir}/cynic-kernel");
    let prev_path = format!("{bin_dir}/cynic-kernel.prev");

    // 1. Check if backup exists
    if !std::path::Path::new(&prev_path).exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "No previous binary found".into(),
            }),
        ));
    }

    // 2. Restore the previous binary by renaming it over the current one
    if let Err(e) = std::fs::rename(&prev_path, &target_path) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to rollback binary: {e}"),
            }),
        ));
    }

    // 3. Spawn background task to trigger restart
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let _ = tokio::process::Command::new("systemctl")
            .arg("--user")
            .arg("restart")
            .arg("cynic-kernel")
            .spawn();
    });

    Ok(Json(serde_json::json!({
        "status": "rollback_scheduled",
        "restarting_in": "500ms"
    })))
}

/// GET /coord/backup — exports the SurrealDB database to a .surql stream.
pub async fn coord_backup_handler()
-> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    let password = std::env::var("SURREALDB_PASS").unwrap_or_default();
    let tmp_path = format!("/tmp/cynic_backup_{}.surql", chrono::Utc::now().timestamp());

    let status = tokio::process::Command::new("surreal")
        .args([
            "export",
            "--endpoint",
            "http://localhost:8000",
            "--namespace",
            "cynic",
            "--database",
            "v2",
            "--username",
            "root",
            "--password",
            &password,
            &tmp_path,
        ])
        .status()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    if !status.success() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Export failed".into(),
            }),
        ));
    }

    let body = tokio::fs::read(&tmp_path).await.unwrap_or_default();
    let _ = tokio::fs::remove_file(&tmp_path).await;

    axum::response::Response::builder()
        .header(
            "Content-Disposition",
            "attachment; filename=\"backup.surql\"",
        )
        .header("Content-Type", "application/octet-stream")
        .body(axum::body::Body::from(body))
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

/// POST /coord/restore — imports a .surql file to SurrealDB.
pub async fn coord_restore_handler(
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let password = std::env::var("SURREALDB_PASS").unwrap_or_default();
    let tmp_path = format!(
        "/tmp/cynic_restore_{}.surql",
        chrono::Utc::now().timestamp()
    );

    if let Err(e) = std::fs::write(&tmp_path, &body) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        ));
    }

    let status = tokio::process::Command::new("surreal")
        .args([
            "import",
            "--endpoint",
            "http://localhost:8000",
            "--namespace",
            "cynic",
            "--database",
            "v2",
            "--username",
            "root",
            "--password",
            &password,
            &tmp_path,
        ])
        .status()
        .await
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp_path);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    let _ = std::fs::remove_file(&tmp_path);

    if !status.success() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Import failed".into(),
            }),
        ));
    }

    Ok(Json(serde_json::json!({ "status": "restored" })))
}

/// POST /coord/upgrade-systemd — updates the .service file and calls daemon-reload.
pub async fn coord_upgrade_systemd_handler(
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let service_dir = format!("{home}/.config/systemd/user");

    let _ = std::fs::create_dir_all(&service_dir);
    let target_path = format!("{service_dir}/cynic-kernel.service");

    if let Err(e) = std::fs::write(&target_path, &body) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        ));
    }

    let status = tokio::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    if !status.success() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "daemon-reload failed".into(),
            }),
        ));
    }

    Ok(Json(serde_json::json!({ "status": "systemd_reloaded" })))
}
