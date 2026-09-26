pub fn spawn_event_consumer(
    event_tx: &tokio::sync::broadcast::Sender<KernelEvent>,
    rest_state: Arc<crate::api::rest::AppState>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
    slack: Option<SlackAlerter>,
) -> JoinHandle<()> {
    spawn_event_consumer_with_liveness(
        event_tx,
        rest_state,
        task_health,
        shutdown,
        constants::EVENT_LIVENESS_INTERVAL,
        slack,
    )
}

