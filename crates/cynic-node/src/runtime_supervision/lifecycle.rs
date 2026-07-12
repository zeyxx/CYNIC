use std::time::Duration;

use process_wrap::tokio::ChildWrapper;
use reqwest::Client;
use tokio_util::sync::CancellationToken;

use crate::announce;
use crate::config;
use crate::config::Config;
use crate::domain::supervision::ExitReason;
use crate::supervise;
use crate::verify;
use crate::websocket;

/// Run one full lifecycle: wait_healthy -> register -> watch.
///
/// Returns the exit reason and the node id if registration succeeded before exit.
pub(crate) async fn run_lifecycle(
    client: &Client,
    cfg: &Config,
    child: &mut Box<dyn ChildWrapper>,
    shutdown: &CancellationToken,
    backoff: &mut supervise::Backoff,
    config_rx: &mut tokio::sync::watch::Receiver<()>,
) -> (ExitReason, Option<String>) {
    if let Err(reason) = wait_healthy(client, cfg, child, shutdown).await {
        return (reason, None);
    }

    match register_with_kernel(client, cfg, child, shutdown).await {
        Ok(id) => {
            backoff.reset();
            let reason = watch(client, cfg, child, &id, shutdown, config_rx).await;
            (reason, Some(id))
        }
        Err(reason) => (reason, None),
    }
}

/// Main supervision loop: heartbeat, health, identity, child-exit, shutdown.
async fn watch(
    client: &Client,
    cfg: &Config,
    child: &mut Box<dyn ChildWrapper>,
    dog_id: &str,
    shutdown: &CancellationToken,
    config_rx: &mut tokio::sync::watch::Receiver<()>,
) -> ExitReason {
    let health_url = config::derive_health_url(&cfg.dog.base_url);
    let models_url = config::derive_models_url(&cfg.dog.base_url);
    let mut heartbeat_tick =
        tokio::time::interval(Duration::from_secs(cfg.kernel.heartbeat_interval_secs));
    let mut health_tick = tokio::time::interval(Duration::from_secs(cfg.health.interval_secs));
    let mut verify_tick =
        tokio::time::interval(Duration::from_secs(cfg.health.verify_interval_secs));
    let mut stimulus_tick = tokio::time::interval(Duration::from_millis(100));
    let mut health_failures: u32 = 0;

    let node_public_key = std::env::var("CYNIC_NODE_PUBLIC_KEY")
        .unwrap_or_else(|_| "stub-public-key-phase-3-4-pending".to_string());

    let api_key = std::env::var("CYNIC_API_KEY").unwrap_or_else(|_| {
        tracing::warn!("CYNIC_API_KEY not set; WebSocket authentication will fail");
        String::new()
    });

    let kernel_ws_url = cfg
        .kernel
        .url
        .replace("http://", "ws://")
        .replace("https://", "wss://")
        + "/events/ws";

    let queue_dir = std::env::var("HOME")
        .ok()
        .map(|h| std::path::PathBuf::from(h).join(".cynic/node"))
        .unwrap_or_else(|| std::path::PathBuf::from(".cynic/node"));

    let ws_client = std::sync::Arc::new(websocket::WebSocketClient::new(
        kernel_ws_url,
        api_key,
        node_public_key,
        cfg.dog.name.clone(),
        queue_dir,
    ));

    if let Err(e) = ws_client.load_persistent_queues().await {
        tracing::warn!("failed to load persistent queues: {e}");
    }

    let ws_client_clone = ws_client.clone();
    let mut ws_handle = tokio::spawn({
        let ws = ws_client_clone.clone();
        async move {
            match ws.run().await {
                Ok(_) => tracing::info!("WebSocket client exited cleanly"),
                Err(e) => tracing::error!("WebSocket client error: {e}"),
            }
        }
    });

    loop {
        let child_wait = child.wait();
        tokio::pin!(child_wait);

        tokio::select! {
            _ = heartbeat_tick.tick() => {
                if let Err(r) = on_heartbeat_tick(client, cfg, dog_id).await {
                    return r;
                }
            }
            _ = health_tick.tick() => {
                if let Some(r) = on_health_tick(client, cfg, &health_url, &mut health_failures).await {
                    return r;
                }
            }
            _ = verify_tick.tick() => {
                if let Some(r) = on_verify_tick(client, cfg, &models_url).await {
                    return r;
                }
            }
            _ = stimulus_tick.tick() => {
                if let Some(pending) = ws_client.dequeue_stimulus().await
                    && let Err(e) = process_stimulus(&ws_client, pending).await
                {
                    tracing::error!("stimulus processing error: {e}");
                }
            }
            status = &mut child_wait => {
                tracing::error!("backend process exited unexpectedly: {status:?}");
                return ExitReason::Crashed;
            }
            Ok(()) = config_rx.changed() => {
                tracing::info!("L3: config file changed on disk - triggering reload");
                return ExitReason::ConfigChanged;
            }
            _ = shutdown.cancelled() => {
                let _ = ws_handle.await;
                return ExitReason::Shutdown;
            }
            _ = &mut ws_handle => {
                tracing::warn!("WebSocket client task exited unexpectedly");
            }
        }
    }
}

async fn on_heartbeat_tick(client: &Client, cfg: &Config, dog_id: &str) -> Result<(), ExitReason> {
    match announce::send_heartbeat(client, &cfg.kernel.url, &cfg.kernel.api_key, dog_id).await {
        announce::HeartbeatResult::Alive => Ok(()),
        announce::HeartbeatResult::Expired => Err(ExitReason::Expired),
        announce::HeartbeatResult::Error(e) => {
            tracing::warn!("heartbeat error: {e}");
            Ok(())
        }
    }
}

async fn on_health_tick(
    client: &Client,
    cfg: &Config,
    health_url: &str,
    health_failures: &mut u32,
) -> Option<ExitReason> {
    if verify::check_health(client, health_url, cfg.health.timeout_secs).await {
        *health_failures = 0;
        None
    } else {
        *health_failures += 1;
        tracing::warn!(
            "health fail ({}/{})",
            health_failures,
            cfg.health.max_failures
        );
        if *health_failures >= cfg.health.max_failures {
            tracing::error!(
                "backend unresponsive after {} failures - will restart",
                cfg.health.max_failures
            );
            return Some(ExitReason::Crashed);
        }
        None
    }
}

async fn on_verify_tick(client: &Client, cfg: &Config, models_url: &str) -> Option<ExitReason> {
    match verify::check_identity(client, models_url, &cfg.dog.model, cfg.health.timeout_secs).await
    {
        verify::IdentityResult::Match => None,
        verify::IdentityResult::Mismatch { expected, actual } => {
            tracing::error!("model mismatch: expected {expected}, got {actual}");
            Some(ExitReason::Mismatch)
        }
        _ => None,
    }
}

async fn process_stimulus(
    ws_client: &std::sync::Arc<websocket::WebSocketClient>,
    stimulus: websocket::PendingStimulus,
) -> Result<(), String> {
    let stimulus_id = stimulus.stimulus_id.clone();

    let (_domain, _content, _context) = match &stimulus.stimulus {
        websocket::NodeMessage::Stimulus {
            domain,
            content,
            context,
            ..
        } => (domain.clone(), content.clone(), context.clone()),
        _ => {
            return Err(format!(
                "stimulus {stimulus_id}: message is not a Stimulus variant"
            ));
        }
    };

    let verdict = serde_json::json!({
        "id": format!("verd_{}", stimulus_id),
        "domain": "unknown",
        "kind": "DETERMINISTIC_FALLBACK",
        "q_score": 0.5,
        "axiom_scores": {
            "FIDELITY": 0.5,
            "PHI": 0.5,
            "VERIFY": 0.5,
            "CULTURE": 0.5,
            "BURN": 0.5,
            "SOVEREIGNTY": 0.5
        },
        "dog_id": "deterministic-fallback-phase-3-2",
        "dog_scores": [0.5],
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    });

    let pending_verdict = websocket::PendingVerdict {
        stimulus_id,
        verdict,
        signature: "stub-signature-phase-3-4-pending".to_string(),
        queued_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
    };

    ws_client.queue_verdict(pending_verdict).await;
    tracing::debug!("stimulus processed and verdict queued");
    Ok(())
}

async fn wait_healthy(
    client: &Client,
    cfg: &Config,
    child: &mut Box<dyn ChildWrapper>,
    shutdown: &CancellationToken,
) -> Result<(), ExitReason> {
    let health_url = config::derive_health_url(&cfg.dog.base_url);
    let deadline =
        tokio::time::Instant::now() + Duration::from_secs(cfg.health.startup_timeout_secs);
    let mut tick = tokio::time::interval(Duration::from_secs(2));

    loop {
        let child_wait = child.wait();
        tokio::pin!(child_wait);

        tokio::select! {
            _ = tick.tick() => {
                if verify::check_health(client, &health_url, cfg.health.timeout_secs).await {
                    tracing::info!("backend healthy - proceeding to registration");
                    return Ok(());
                }
                if tokio::time::Instant::now() >= deadline {
                    tracing::error!(
                        "startup health timeout after {}s",
                        cfg.health.startup_timeout_secs
                    );
                    return Err(ExitReason::Crashed);
                }
                tracing::debug!("backend not yet healthy, retrying...");
            }
            _ = &mut child_wait => {
                tracing::error!("backend exited during startup health check");
                return Err(ExitReason::Crashed);
            }
            _ = shutdown.cancelled() => return Err(ExitReason::Shutdown),
        }
    }
}

async fn register_with_kernel(
    client: &Client,
    cfg: &Config,
    child: &mut Box<dyn ChildWrapper>,
    shutdown: &CancellationToken,
) -> Result<String, ExitReason> {
    let payload = build_registration_payload(cfg);
    let max_calibration_attempts: u32 = 3;
    let mut calibration_attempts: u32 = 0;

    loop {
        let child_wait = child.wait();
        tokio::pin!(child_wait);

        tokio::select! {
            result = announce::try_register(client, &cfg.kernel.url, &cfg.kernel.api_key, &payload) => {
                match result {
                    Ok(resp) => {
                        tracing::info!(
                            dog_id = %resp.dog_id,
                            roster_size = resp.roster_size,
                            "registered with kernel"
                        );
                        return Ok(resp.dog_id);
                    }
                    Err(announce::RegisterError::Collision) => {
                        tracing::info!(
                            dog_name = %cfg.dog.name,
                            "registration collision - Dog already in kernel roster, trying heartbeat"
                        );
                        match announce::send_heartbeat(client, &cfg.kernel.url, &cfg.kernel.api_key, &cfg.dog.name).await {
                            announce::HeartbeatResult::Alive => {
                                tracing::info!(
                                    dog_id = %cfg.dog.name,
                                    "heartbeat accepted - adopting existing Dog"
                                );
                                return Ok(cfg.dog.name.clone());
                            }
                            announce::HeartbeatResult::Expired => {
                                tracing::warn!("Dog existed but TTL expired - retrying registration");
                                tokio::time::sleep(Duration::from_secs(5)).await;
                            }
                            announce::HeartbeatResult::Error(e) => {
                                tracing::warn!("heartbeat fallback failed: {e} - retrying registration");
                                tokio::time::sleep(Duration::from_secs(5)).await;
                            }
                        }
                    }
                    Err(announce::RegisterError::CalibrationFail(body)) => {
                        calibration_attempts += 1;
                        tracing::warn!(attempt = calibration_attempts, max = max_calibration_attempts, "calibration failed: {body}");
                        if calibration_attempts >= max_calibration_attempts {
                            return Err(ExitReason::Fatal(format!(
                                "calibration failed after {max_calibration_attempts} attempts: {body}"
                            )));
                        }
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    Err(announce::RegisterError::Transient(msg)) => {
                        tracing::warn!("transient registration error (will retry): {msg}");
                        tokio::time::sleep(Duration::from_secs(10)).await;
                    }
                }
            }
            _ = &mut child_wait => {
                tracing::error!("backend exited during registration");
                return Err(ExitReason::Crashed);
            }
            _ = shutdown.cancelled() => return Err(ExitReason::Shutdown),
        }
    }
}

fn build_registration_payload(cfg: &Config) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "name":         cfg.dog.name,
        "model":        cfg.dog.model,
        "base_url":     cfg.dog.base_url,
        "context_size": cfg.dog.context_size,
        "timeout_secs": cfg.dog.timeout_secs,
    });
    if let Some(ref key) = cfg.dog.api_key {
        payload["api_key"] = serde_json::Value::String(key.clone());
    }
    payload
}
