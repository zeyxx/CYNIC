pub fn spawn_probe_scheduler(
    probes: Vec<Arc<dyn Probe>>,
    _storage: Arc<dyn StoragePort>,
    environment: Arc<std::sync::RwLock<Option<EnvironmentSnapshot>>>,
    organ: Arc<crate::organ::InferenceOrgan>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    use crate::infra::probes::ProbeScheduler;

    let mut scheduler = ProbeScheduler::new(probes);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(constants::PROBE_SCHEDULER_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick (consistent with all other tasks)

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Probe scheduler stopped");
                    break;
                }
                _ = interval.tick() => {
                    // K6: per-probe timeouts inside scheduler.tick() satisfy K6.
                    // No outer timeout needed — each probe's .await is bounded.
                    if let Some(snapshot) = scheduler.tick().await {
                        if let Ok(mut env_guard) = environment.write() {
                            match env_guard.as_mut() {
                                Some(existing) => {
                                    // Merge new probe results into existing snapshot.
                                    // Probes with longer intervals stay visible until their next tick.
                                    for new_probe in snapshot.probes {
                                        if let Some(pos) = existing.probes.iter().position(|p| p.name == new_probe.name) {
                                            existing.probes[pos] = new_probe;
                                        } else {
                                            existing.probes.push(new_probe);
                                        }
                                    }
                                    existing.timestamp = snapshot.timestamp;
                                    existing.overall = EnvironmentSnapshot::worst_status(&existing.probes);
                                }
                                None => {
                                    *env_guard = Some(snapshot.clone());
                                }
                            }
                        }

                        // Use the combined snapshot for organ degradation logic
                        let current_snapshot = environment.read().ok().and_then(|g| g.clone());
                        if let Some(snap) = current_snapshot {
                            for probe in &snap.probes {
                                if let ProbeDetails::Fleet(ref fleet) = probe.details {
                                    for dog in &fleet.dogs {
                                        if dog.model_mismatch {
                                            organ.degrade_backend(
                                                &dog.dog_name,
                                                format!(
                                                    "model mismatch: expected {:?}, actual {:?}",
                                                    dog.expected_model, dog.actual_model
                                                ),
                                            );
                                        } else {
                                            organ.promote_if_gate_clear(&dog.dog_name);
                                        }
                                    }
                                }
                            }
                        }

                        task_health.touch_probe_scheduler();
                    }
                }
            }
        }
    })
}

