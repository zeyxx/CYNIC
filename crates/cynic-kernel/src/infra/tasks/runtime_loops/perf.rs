pub fn spawn_dog_perf_flush_loop(
    rest_state: Arc<crate::api::rest::AppState>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(constants::DOG_PERF_FLUSH_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Dog performance flush loop stopped");
                    break;
                }
                _ = interval.tick() => {
                    // Flush all aggregated Dog performance metrics to routing calculator
                    rest_state
                        .dog_perf_collector
                        .flush_all(&rest_state.routing_calc);

                    tracing::debug!("Dog performance metrics flushed to routing calculator");
                    task_health.touch_dog_perf_flush();
                }
            }
        }
    })
}

