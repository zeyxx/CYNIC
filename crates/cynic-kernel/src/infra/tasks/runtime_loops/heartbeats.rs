pub fn spawn_dog_heartbeat_loop(
    rest_state: Arc<crate::api::rest::AppState>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // K6: 40s interval matches cynic-node heartbeat cadence
        let mut interval = tokio::time::interval(constants::DOG_HEARTBEAT_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Dog heartbeat loop stopped");
                    break;
                }
                _ = interval.tick() => {
                    // Count Dogs needing heartbeat (TTL < 60s means heartbeat within safe margin)
                    let to_refresh: Vec<String> = match rest_state.registered_dogs.read() {
                        Ok(map) => map
                            .iter()
                            .filter(|(_, dog)| {
                                let elapsed = dog.last_heartbeat.elapsed().as_secs();
                                elapsed > dog.ttl_secs.saturating_sub(constants::HEARTBEAT_TTL_MARGIN)
                            })
                            .map(|(id, _)| id.clone())
                            .collect(),
                        Err(e) => {
                            tracing::warn!(error = %e, "registered_dogs RwLock poisoned — skipping heartbeat");
                            vec![]
                        }
                    };

                    if !to_refresh.is_empty() {
                        let mut updated = 0;
                        for dog_id in to_refresh {
                            match rest_state.registered_dogs.write() {
                                Ok(mut map) => {
                                    if let Some(entry) = map.get_mut(&dog_id) {
                                        entry.last_heartbeat = std::time::Instant::now();
                                        updated += 1;
                                        tracing::debug!(dog_id = %dog_id, "Dog heartbeat refreshed");
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        error = %e,
                                        dog_id = %dog_id,
                                        "registered_dogs write lock poisoned — cannot refresh heartbeat"
                                    );
                                }
                            }
                        }
                        if updated > 0 {
                            tracing::info!(count = updated, "Dogs heartbeat refreshed");
                        }
                    }

                    task_health.touch_dog_heartbeat();
                }
            }
        }
    })
}

