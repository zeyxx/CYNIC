use tokio_util::sync::CancellationToken;

/// Install OS signal handlers and cancel `shutdown` when a signal arrives.
pub fn install_signal_handler(shutdown: CancellationToken) {
    #[cfg(unix)]
    tokio::spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Err(error) => {
                tracing::warn!("SIGTERM handler install failed ({error}) — only SIGINT will work");
                tokio::signal::ctrl_c().await.ok();
            }
            Ok(mut sigterm) => {
                tokio::select! {
                    _ = sigterm.recv() => tracing::info!("SIGTERM received — shutting down"),
                    _ = tokio::signal::ctrl_c() => tracing::info!("SIGINT received — shutting down"),
                }
            }
        }
        shutdown.cancel();
    });

    #[cfg(windows)]
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Ctrl-C received — shutting down");
        shutdown.cancel();
    });
}
