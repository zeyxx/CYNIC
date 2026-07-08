use std::process::ExitCode;

use process_wrap::tokio::ChildWrapper;
use reqwest::Client;
use tokio_util::sync::CancellationToken;

use crate::announce;
use crate::config;
use crate::config::Config;
use crate::config_watch;
use crate::run_lifecycle;
use crate::supervise;
use crate::ExitReason;

/// The main supervision loop. Returns `ExitCode::SUCCESS` on clean shutdown,
/// `ExitCode::FAILURE` on unrecoverable errors.
///
/// Owns the config so it can be reloaded from disk on model mismatch (L2 auto-heal).
pub(crate) async fn run_node(
    client: &Client,
    config_path: &str,
    mut cfg: Config,
    shutdown: &CancellationToken,
) -> ExitCode {
    let (mut config_rx, _watcher) = match config_watch::spawn_config_watcher(config_path) {
        Ok((rx, watcher)) => (rx, Some(watcher)),
        Err(e) => {
            tracing::warn!(
                "L3 file watcher unavailable ({e}) — falling back to L2 mismatch detection"
            );
            let (_, rx) = tokio::sync::watch::channel(());
            (rx, None)
        }
    };

    tokio::spawn(crate::system_probe::run_system_probe(
        client.clone(),
        cfg.kernel.url.clone(),
        cfg.kernel.api_key.clone(),
        cfg.dog.name.clone(),
        60,
    ));

    let mut state = NodeState {
        needs_spawn: true,
        child: None,
        backoff: supervise::Backoff::from_config(&cfg.restart),
        dog_id: None,
    };

    let exit_code = loop {
        if state.needs_spawn {
            if let Some(id) = state.dog_id.take() {
                deregister(client, &cfg, &id).await;
            }
            match spawn_backend(&cfg).await {
                Ok(c) => {
                    state.child = Some(c);
                    state.backoff.record_start();
                }
                Err(e) => {
                    tracing::error!("spawn failed: {e}");
                    break ExitCode::FAILURE;
                }
            }
        }

        let Some(c) = state.child.as_mut() else {
            tracing::error!("internal: child is None after spawn");
            break ExitCode::FAILURE;
        };

        let (reason, id) =
            run_lifecycle(client, &cfg, c, shutdown, &mut state.backoff, &mut config_rx).await;
        state.dog_id = id;

        if matches!(&reason, ExitReason::Mismatch | ExitReason::ConfigChanged) {
            match config::load(config_path) {
                Ok(new_cfg) => {
                    let model_changed = new_cfg.dog.model != cfg.dog.model;
                    let command_changed = new_cfg.process.command != cfg.process.command;
                    let name_changed = new_cfg.dog.name != cfg.dog.name;
                    if model_changed || command_changed || name_changed {
                        tracing::info!(
                            old_model = %cfg.dog.model,
                            new_model = %new_cfg.dog.model,
                            old_name = %cfg.dog.name,
                            new_name = %new_cfg.dog.name,
                            "L2 auto-heal: config changed on disk — adopting new config"
                        );
                        cfg = new_cfg;
                        state.backoff.reset();
                    } else {
                        tracing::warn!(
                            "L2: config unchanged on disk — runtime mismatch, restarting backend"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("L2: config reload failed ({e}) — using existing config");
                }
            }
        }

        match handle_exit_reason(reason, &cfg, &mut state, shutdown, client).await {
            LoopAction::Continue => {}
            LoopAction::Break(code) => break code,
        }
    };

    tracing::info!("cynic-node stopped");
    exit_code
}

enum LoopAction {
    Continue,
    Break(ExitCode),
}

#[derive(Debug)]
struct NodeState {
    needs_spawn: bool,
    child: Option<Box<dyn ChildWrapper>>,
    backoff: supervise::Backoff,
    dog_id: Option<String>,
}

async fn handle_exit_reason(
    reason: ExitReason,
    cfg: &Config,
    state: &mut NodeState,
    shutdown: &CancellationToken,
    client: &Client,
) -> LoopAction {
    match reason {
        ExitReason::Shutdown => {
            if let Some(id) = state.dog_id.take() {
                deregister(client, cfg, &id).await;
            }
            stop_child(&mut state.child, cfg).await;
            LoopAction::Break(ExitCode::SUCCESS)
        }
        ExitReason::Expired => {
            tracing::warn!("kernel expired this dog — will re-register without respawn");
            state.dog_id = None;
            state.needs_spawn = false;
            LoopAction::Continue
        }
        ExitReason::Crashed => {
            state.dog_id = None;
            state.needs_spawn = true;
            state.backoff.reset_if_stable();
            if state.backoff.exhausted() {
                tracing::error!(
                    "max restart attempts ({}) reached — giving up",
                    cfg.restart.max_attempts
                );
                stop_child(&mut state.child, cfg).await;
                return LoopAction::Break(ExitCode::FAILURE);
            }
            tracing::info!("backend crashed — waiting before restart");
            state.backoff.wait_or_shutdown(shutdown).await;
            LoopAction::Continue
        }
        ExitReason::Mismatch | ExitReason::ConfigChanged => {
            if let Some(id) = state.dog_id.take() {
                deregister(client, cfg, &id).await;
            }
            stop_child(&mut state.child, cfg).await;
            state.needs_spawn = true;
            LoopAction::Continue
        }
        ExitReason::Fatal(e) => {
            tracing::error!("fatal: {e}");
            if let Some(id) = state.dog_id.take() {
                deregister(client, cfg, &id).await;
            }
            stop_child(&mut state.child, cfg).await;
            LoopAction::Break(ExitCode::FAILURE)
        }
    }
}

async fn stop_child(child: &mut Option<Box<dyn ChildWrapper>>, cfg: &Config) {
    if let Some(c) = child.as_mut() {
        let _ = supervise::graceful_stop(c, cfg.process.stop_timeout_secs).await;
    }
}

async fn deregister(client: &Client, cfg: &Config, dog_id: &str) {
    announce::try_deregister(client, &cfg.kernel.url, &cfg.kernel.api_key, dog_id).await;
}

async fn spawn_backend(cfg: &Config) -> std::io::Result<Box<dyn ChildWrapper>> {
    tracing::info!("spawning backend: {:?}", cfg.process.command);
    supervise::spawn_with_retries(
        &cfg.process.command,
        cfg.process.working_dir.as_deref(),
        &cfg.process.env,
        3,
    )
    .await
}
