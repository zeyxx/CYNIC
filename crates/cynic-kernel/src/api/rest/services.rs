//! REST handlers for the OS-level service registry — /services process table.
//!
//! Every running organism service registers here on startup and heartbeats to stay visible.
//! This is the kernel's /proc equivalent:
//!   desired state  = infra/registry.json (static SSOT, human-edited)
//!   observed state = this registry        (dynamic, TTL-evicted)
//!   delta          = desired − observed   = what's missing or unknown
//!
//! Pattern mirrors /dogs exactly: register → heartbeat → unregister.
//! Auth: Role::Organ (same token services use for /observe and /coord).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use super::types::{
    AppState, ErrorResponse, RegisterServiceRequest, RegisterServiceResponse, RegisteredService,
    ServiceHeartbeatResponse, ServiceSnapshot, ServiceUnregisterResponse,
};

/// Default TTL for a registered service.
/// A service that misses one heartbeat cycle (60s) plus grace period (60s) is evicted.
const DEFAULT_TTL_SECS: u64 = 120;

/// Maximum allowed TTL — prevents services from registering with infinite TTL.
const MAX_TTL_SECS: u64 = 600;

// ── GET /services ─────────────────────────────────────────────

/// GET /services — list all currently registered services with their liveness snapshot.
///
/// Returns the kernel's observed process table. Compare against infra/registry.json
/// desired state to compute delta (what's declared but not observed).
pub async fn list_services_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ServiceSnapshot>> {
    let now = std::time::Instant::now();
    let map = state
        .registered_services
        .read()
        .unwrap_or_else(|e| e.into_inner());

    let mut services: Vec<ServiceSnapshot> = map
        .values()
        .map(|svc| ServiceSnapshot {
            service_id: svc.service_id.clone(),
            tier: svc.tier,
            node: svc.node.clone(),
            capabilities: svc.capabilities.clone(),
            version: svc.version.clone(),
            uptime_secs: now.duration_since(svc.registered_at).as_secs(),
            last_heartbeat_secs_ago: now.duration_since(svc.last_heartbeat).as_secs(),
            ttl_secs: svc.ttl_secs,
        })
        .collect();

    // Stable order: tier asc, then service_id alpha — makes diffs easy to read.
    services.sort_by(|a, b| a.tier.cmp(&b.tier).then(a.service_id.cmp(&b.service_id)));
    Json(services)
}

// ── POST /services/register ───────────────────────────────────

/// POST /services/register — service announces itself to the kernel process table.
///
/// Called once at service startup (from its own code, not from a script).
/// Idempotent: re-registration resets the TTL and updates metadata.
/// Emits `KernelEvent::ServiceRegistered` on first registration.
pub async fn register_service_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterServiceRequest>,
) -> Result<Json<RegisterServiceResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate service_id
    let service_id = req.service_id.trim().to_string();
    if service_id.is_empty() || service_id.len() > 128 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "service_id must be 1-128 characters".into(),
            }),
        ));
    }

    // Validate tier
    if req.tier > 4 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "tier must be 0-4 (0=kernel, 1=nervoussystem, 2=organs, 3=secondary, 4=ui)"
                    .into(),
            }),
        ));
    }

    let ttl_secs = req.ttl_secs.unwrap_or(DEFAULT_TTL_SECS).min(MAX_TTL_SECS);
    let now = std::time::Instant::now();

    let is_new = {
        let mut map = state
            .registered_services
            .write()
            .unwrap_or_else(|e| e.into_inner());

        let is_new = !map.contains_key(&service_id);

        // Preserve original registration time on re-register.
        let registered_at = if is_new {
            now
        } else {
            map.get(&service_id).map(|s| s.registered_at).unwrap_or(now)
        };

        // Upsert: re-registration updates metadata + resets heartbeat.
        map.insert(
            service_id.clone(),
            RegisteredService {
                service_id: service_id.clone(),
                tier: req.tier,
                node: req.node.clone(),
                capabilities: req.capabilities.clone(),
                version: req.version.clone(),
                registered_at,
                last_heartbeat: now,
                ttl_secs,
            },
        );
        is_new
    };

    if is_new {
        tracing::info!(
            service_id = %service_id,
            tier = req.tier,
            node = %req.node,
            "service registered — process table updated"
        );
        let _ = state
            .event_tx
            .send(crate::domain::events::KernelEvent::ServiceRegistered {
                service_id: service_id.clone(),
                tier: req.tier,
                node: req.node,
            });
    } else {
        tracing::debug!(service_id = %service_id, "service re-registered (TTL reset)");
    }

    Ok(Json(RegisterServiceResponse {
        service_id,
        status: if is_new {
            "registered"
        } else {
            "re-registered"
        }
        .into(),
        ttl_secs,
    }))
}

// ── POST /services/{id}/heartbeat ─────────────────────────────

/// POST /services/{id}/heartbeat — refresh TTL for a registered service.
///
/// Called periodically (every ttl_secs/2) from within the service's own code.
/// No script, no cron — the heartbeat is a native async task inside the service.
pub async fn service_heartbeat_handler(
    State(state): State<Arc<AppState>>,
    Path(service_id): Path<String>,
) -> Result<Json<ServiceHeartbeatResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut map = state
        .registered_services
        .write()
        .unwrap_or_else(|e| e.into_inner());

    if let Some(entry) = map.get_mut(&service_id) {
        entry.last_heartbeat = std::time::Instant::now();
        let ttl_remaining_secs = entry.ttl_secs;
        return Ok(Json(ServiceHeartbeatResponse {
            service_id,
            status: "alive".into(),
            ttl_remaining_secs,
        }));
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: format!(
                "service '{}' not registered — call POST /services/register first",
                service_id
            ),
        }),
    ))
}

// ── DELETE /services/{id} ─────────────────────────────────────

/// DELETE /services/{id} — explicit service shutdown announce.
///
/// Called from the service's SIGTERM handler before exit.
/// Emits `KernelEvent::ServiceDown` with reason "explicit_unregister".
/// No script — the service's own shutdown code calls this.
pub async fn unregister_service_handler(
    State(state): State<Arc<AppState>>,
    Path(service_id): Path<String>,
) -> Result<Json<ServiceUnregisterResponse>, (StatusCode, Json<ErrorResponse>)> {
    let removed = {
        let mut map = state
            .registered_services
            .write()
            .unwrap_or_else(|e| e.into_inner());
        map.remove(&service_id).is_some()
    };

    if !removed {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("service '{}' not found in process table", service_id),
            }),
        ));
    }

    tracing::info!(service_id = %service_id, "service unregistered (explicit shutdown)");
    let _ = state
        .event_tx
        .send(crate::domain::events::KernelEvent::ServiceDown {
            service_id: service_id.clone(),
            reason: "explicit_unregister".into(),
        });

    Ok(Json(ServiceUnregisterResponse {
        service_id,
        status: "unregistered".into(),
    }))
}

// ── TTL eviction (called by background runtime loop) ─────────

/// Evict services whose TTL has expired. Called by the background task scheduler.
/// Emits `KernelEvent::ServiceDown` for each evicted service.
///
/// This is the kernel's equivalent of a reaper process.
/// Must be called periodically (every 30s) from `infra/tasks/runtime_loops.rs`.
pub fn evict_stale_services(state: &AppState) {
    let now = std::time::Instant::now();
    let mut evicted = Vec::new();

    {
        let mut map = state
            .registered_services
            .write()
            .unwrap_or_else(|e| e.into_inner());

        map.retain(|service_id, entry| {
            let age = now.duration_since(entry.last_heartbeat).as_secs();
            if age > entry.ttl_secs {
                evicted.push(service_id.clone());
                false
            } else {
                true
            }
        });
    }

    for service_id in evicted {
        tracing::warn!(service_id = %service_id, "service TTL expired — evicted from process table");
        let _ = state
            .event_tx
            .send(crate::domain::events::KernelEvent::ServiceDown {
                service_id,
                reason: "ttl_expired".into(),
            });
    }
}
