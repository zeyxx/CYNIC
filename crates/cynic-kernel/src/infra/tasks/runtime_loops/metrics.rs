pub fn spawn_storage_metrics_emitter(
    storage: Arc<dyn StoragePort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Storage metrics emitter stopped");
                    break;
                }
                _ = interval.tick() => {
                    // K15: snapshot per-table metrics and emit observations
                    // Extracts table-level latency data to detect bottlenecks
                    #[allow(clippy::collapsible_if)]
                    // WHY: the nested if distinguishes a retrieval error from empty results (two failure modes)
                    match storage.get_table_op_metrics().await {
                        Ok(table_metrics) => {
                            if !table_metrics.is_empty() {
                            let mut tables_snapshot = Vec::new();
                            for metric in table_metrics {
                                let avg_latency_ms = metric.total_latency_us
                                    .checked_div(metric.count.max(1))
                                    .unwrap_or(0) as f64 / 1000.0;

                                let metric_json = serde_json::json!({
                                    "table": metric.table.clone(),
                                    "operation": metric.operation.clone(),
                                    "count": metric.count,
                                    "errors": metric.errors,
                                    "avg_latency_ms": avg_latency_ms,
                                    "min_latency_ms": (metric.min_latency_us as f64 / 1000.0),
                                    "max_latency_ms": (metric.max_latency_us as f64 / 1000.0),
                                    "slow_count": metric.slow_count,
                                });
                                tables_snapshot.push(metric_json);

                                tracing::debug!(
                                    table = %metric.table,
                                    operation = %metric.operation,
                                    count = metric.count,
                                    errors = metric.errors,
                                    avg_latency_ms = avg_latency_ms,
                                    min_latency_ms = (metric.min_latency_us as f64 / 1000.0),
                                    max_latency_ms = (metric.max_latency_us as f64 / 1000.0),
                                    slow_count = metric.slow_count,
                                    "storage metrics snapshot"
                                );
                            }

                            // K15: Emit observation for storage bottleneck detection
                            // Extracted to separate function to avoid LLVM codegen complexity
                            tracing::info!(
                                snapshot_count = tables_snapshot.len(),
                                "attempting to emit storage metrics observation"
                            );
                            if let Err(e) = emit_storage_metrics_observation(&storage, tables_snapshot).await {
                                tracing::warn!(error = %e, "failed to emit storage metrics observation");
                            } else {
                                tracing::info!("storage metrics observation emitted successfully");
                            }
                            } else {
                                tracing::info!(count = 0, "no storage metrics to emit (table_op_metrics empty)");
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "failed to retrieve storage metrics snapshot");
                        }
                    }
                    task_health.touch_storage_metrics();
                }
            }
        }
    })
}

