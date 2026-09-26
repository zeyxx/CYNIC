pub fn spawn_pattern_healing_alerter(
    kernel_addr: String,
    api_key: String,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
    slack: Option<SlackAlerter>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let Some(slack) = slack else {
            klog!("[PatternHealing] Slack not configured — alerter disabled");
            return;
        };

        let mut processed_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut poll_interval = tokio::time::interval(std::time::Duration::from_secs(5));
        poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        poll_interval.tick().await; // skip first tick

        let client = reqwest::Client::new();
        let mut headers = reqwest::header::HeaderMap::new();
        let _ = format!("Bearer {api_key}")
            .parse::<reqwest::header::HeaderValue>()
            .map(|h| headers.insert("Authorization", h));

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[PatternHealing] Alerter stopped");
                    break;
                }
                _ = poll_interval.tick() => {
                    // Query /observations?consumer=pattern_healing for unhandled observations
                    let url = format!("http://{kernel_addr}/observations?consumer=pattern_healing&limit=50");
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        client.get(&url).headers(headers.clone()).send(),
                    )
                    .await
                    {
                        Ok(Ok(resp)) if resp.status().is_success() => {
                            match resp.json::<Vec<serde_json::Value>>().await {
                                Ok(observations) => {
                                    for obs_val in observations {
                                        let obs_id = obs_val
                                            .get("id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown")
                                            .to_string();
                                        let obs_status = obs_val
                                            .get("status")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("info")
                                            .to_string();
                                        let obs_domain = obs_val
                                            .get("domain")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown")
                                            .to_string();
                                        let obs_context = obs_val
                                            .get("context")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();

                                        // Idempotency: skip if we've already processed this observation
                                        if !processed_ids.insert(obs_id.clone()) {
                                            continue;
                                        }

                                        // Alert only on critical patterns
                                        if obs_status == "critical" {
                                            let alert_msg = format!("🔺 **CYNIC Healing** [{obs_domain}]\n{obs_context}");

                                            match slack.send(&alert_msg).await {
                                                Ok(_) => {
                                                    klog!("[PatternHealing] Alert sent ({obs_domain})");
                                                    task_health.touch_pattern_healing_alerter();
                                                }
                                                Err(e) => {
                                                    tracing::warn!(error = %e, "Slack send failed");
                                                    // Don't remove from processed_ids; retry on next tick
                                                    processed_ids.remove(&obs_id);
                                                }
                                            }
                                        } else {
                                            // Non-critical — don't alert, but mark as processed
                                            task_health.touch_pattern_healing_alerter();
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(error = %e, "Failed to parse observations JSON");
                                }
                            }
                        }
                        Ok(Ok(resp)) => {
                            tracing::warn!(status = resp.status().as_u16(), "observations endpoint error");
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(error = %e, "HTTP request failed");
                        }
                        Err(_) => {
                            tracing::warn!("observations query timeout");
                        }
                    }
                }
            }
        }
    })
}

