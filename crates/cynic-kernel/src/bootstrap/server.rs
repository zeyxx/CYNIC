//! Runtime serving surface for REST + UDS lifecycle.
//!
//! This module owns host-binding concerns (TCP/UDS listeners, graceful shutdown,
//! and final usage flush/drain sequence). It keeps `main.rs` focused on composition.

use std::sync::Arc;
use std::time::Duration;

use axum::{Router, serve};
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;

use crate::api::rest::AppState;
use crate::domain;
use crate::infra;

pub struct RestServeInputs {
    pub rest_app: Router,
    pub rest_addr: String,
    pub rest_state: Arc<AppState>,
    pub usage_tracker: Arc<tokio::sync::Mutex<domain::usage::DogUsageTracker>>,
    pub task_health: Arc<infra::task_health::TaskHealth>,
    pub storage: Arc<dyn domain::storage::StoragePort>,
    pub shutdown: CancellationToken,
}

impl std::fmt::Debug for RestServeInputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestServeInputs")
            .field("rest_addr", &self.rest_addr)
            .field(
                "rest_state_has_bg_tasks",
                &self.rest_state.bg_tasks.is_closed(),
            )
            .field(
                "task_health_ptr",
                &(Arc::as_ptr(&self.task_health) as *const std::ffi::c_void),
            )
            .finish()
    }
}

#[derive(Debug)]
pub struct RestServeOutputs {
    pub uds_path: String,
}

pub async fn run_rest_servers(
    inputs: RestServeInputs,
) -> Result<RestServeOutputs, Box<dyn std::error::Error>> {
    let RestServeInputs {
        rest_app,
        rest_addr,
        rest_state,
        usage_tracker,
        task_health,
        storage,
        shutdown,
    } = inputs;

    let uds_path = "/tmp/cynic-kernel.sock".to_string();
    {
        let rs = Arc::clone(&rest_state);
        let th = Arc::clone(&task_health);
        let sd = shutdown.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
            interval.tick().await;
            loop {
                tokio::select! {
                    _ = sd.cancelled() => {
                        klog!("[SHUTDOWN] Rate limiter eviction stopped");
                        break;
                    }
                    _ = interval.tick() => {
                        match tokio::time::timeout(Duration::from_secs(5), rs.rate_limiter.evict_stale()).await {
                            Ok(()) => {}
                            Err(_) => tracing::warn!("rate_limiter evict_stale timed out (5s)"),
                        }
                        match tokio::time::timeout(Duration::from_secs(5), rs.judge_limiter.evict_stale()).await {
                            Ok(()) => {}
                            Err(_) => tracing::warn!("judge_limiter evict_stale timed out (5s)"),
                        }
                        th.touch_rate_eviction();
                    }
                }
            }
        });
    }

    let rest_listener = tokio::net::TcpListener::bind(&rest_addr).await?;
    let _ = std::fs::remove_file(&uds_path);
    let uds_listener = tokio::net::UnixListener::bind(&uds_path)?;
    let _ = std::fs::set_permissions(
        &uds_path,
        std::os::unix::fs::PermissionsExt::from_mode(0o600),
    );

    crate::klog!("Setting up runtime serving on rest_addr={rest_addr} uds_path={uds_path}");
    infra::tasks::spawn_signal_handler(shutdown.clone());

    let uds_rest_app = rest_app.clone();
    {
        let mut make_service = uds_rest_app.into_make_service();
        let shutdown_uds = shutdown.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_uds.cancelled() => {
                        klog!("[SHUTDOWN] UDS server drained");
                        break;
                    }
                    accept_result = uds_listener.accept() => {
                            if let Ok((socket, _addr)) = accept_result {
                            let io = hyper_util::rt::TokioIo::new(socket);
                            let Ok(tower_service) = tower::Service::call(&mut make_service, ()).await;
                            let svc = hyper::service::service_fn(move |req| {
                                let mut svc = tower_service.clone();
                                async move { tower::Service::call(&mut svc, req).await }
                            });
                            tokio::spawn(async move {
                                if let Err(e) = hyper::server::conn::http1::Builder::new()
                                    .serve_connection(io, svc)
                                    .with_upgrades()
                                    .await
                                {
                                    tracing::debug!("UDS Connection error: {}", e);
                                }
                            });
                        }
                    }
                }
            }
        });
    }

    if let Err(e) = serve(
        rest_listener,
        rest_app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown.clone().cancelled_owned())
    .await
    {
        tracing::error!(error = %e, "REST/TCP server fatal error");
    }

    klog!("[SHUTDOWN] REST server drained — flushing state");
    infra::tasks::flush_usage_on_shutdown(&storage, &usage_tracker).await;

    rest_state.bg_tasks.close();
    match tokio::time::timeout(Duration::from_secs(5), rest_state.bg_tasks.wait()).await {
        Ok(()) => klog!("[SHUTDOWN] Background tasks drained ({} completed)", 0),
        Err(_) => tracing::warn!("background tasks did not drain within 5s"),
    }

    Ok(RestServeOutputs { uds_path })
}
