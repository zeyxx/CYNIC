//! Pure rules and trend analysis for introspection.

use crate::domain::organ::{MetricValue, OrganSnapshot as SenseSnapshot};
use crate::domain::state_log::StateBlock;
use crate::introspection::types::{Alert, SAVINGS_PCT_THRESHOLD};

/// Check sense snapshots for anomalies. Pure function — no I/O.
pub fn check_sense_health(snapshots: &[(String, SenseSnapshot)]) -> Vec<Alert> {
    let mut alerts = Vec::new();

    for (name, snap) in snapshots {
        for metric in &snap.metrics {
            match (name.as_str(), metric.key.as_str()) {
                ("rtk", "savings_pct") => {
                    if let MetricValue::F64(pct) = &metric.value
                        && *pct < SAVINGS_PCT_THRESHOLD
                    {
                        alerts.push(Alert {
                            kind: "metabolism_anomaly",
                            message: format!(
                                "RTK savings {pct:.1}% below threshold {SAVINGS_PCT_THRESHOLD:.1}% — token filtering degraded"
                            ),
                            severity: if *pct < 20.0 { "critical" } else { "warning" },
                        });
                    }
                }
                ("tailscale", key) if key.starts_with("node_") && key.ends_with("_online") => {
                    if let MetricValue::Bool(false) = &metric.value {
                        let node = key
                            .strip_prefix("node_")
                            .and_then(|s| s.strip_suffix("_online"))
                            .unwrap_or(key);
                        alerts.push(Alert {
                            kind: "fleet_drift",
                            message: format!("Fleet node '{node}' offline"),
                            severity: "warning",
                        });
                    }
                }
                _ => {}
            }
        }
    }

    alerts
}

/// Whether a source name represents a permanent entity (organ/service) vs ephemeral session.
/// Permanent sources generate alerts on silence. Ephemeral sessions don't.
pub fn is_permanent_source(s: &str) -> bool {
    classify_source_tier(s) == "permanent"
}

/// Classify an agent_id into a source tier at write time (E1).
/// Single source of truth — called by POST /observe and by backfill.
/// Returns: "permanent" (organs, services), "cron" (scheduled jobs), "session" (ephemeral).
pub fn classify_source_tier(agent_id: &str) -> &'static str {
    if agent_id.is_empty() || agent_id == "unknown" {
        return "session";
    }
    if agent_id.starts_with("cron-") {
        return "cron";
    }
    if agent_id.starts_with("hermes")
        || agent_id.starts_with("x-")
        || agent_id == "kernel"
        || agent_id == "nightshift"
        || agent_id == "watchlist"
        || agent_id.starts_with("organ-")
    {
        return "permanent";
    }
    "session"
}

/// Analyze two consecutive state blocks for trends. Pure function — no I/O.
/// Returns alerts for: Dog degradation, system regression, organ silence.
pub fn analyze_state_log_trends(prev: &StateBlock, curr: &StateBlock) -> Vec<Alert> {
    let mut alerts = Vec::new();

    // Dog degradation: success_rate drop > 20pp
    for curr_dog in &curr.dogs {
        if let Some(prev_dog) = prev.dogs.iter().find(|d| d.id == curr_dog.id) {
            let delta = prev_dog.success_rate - curr_dog.success_rate;
            if delta > 0.20 {
                alerts.push(Alert {
                    kind: "dog_degradation_trend",
                    message: format!(
                        "Dog '{}' success rate dropped {:.0}pp ({:.1}% → {:.1}%) in 60s",
                        curr_dog.id,
                        delta * 100.0,
                        prev_dog.success_rate * 100.0,
                        curr_dog.success_rate * 100.0,
                    ),
                    severity: "warning",
                });
            }
        }
    }

    // System status regression: sovereign → degraded/critical
    if prev.system.status == "sovereign" && curr.system.status != "sovereign" {
        alerts.push(Alert {
            kind: "system_regression",
            message: format!(
                "System regressed: {} → {} (dogs {}/{} → {}/{})",
                prev.system.status,
                curr.system.status,
                prev.system.healthy_dogs,
                prev.system.total_dogs,
                curr.system.healthy_dogs,
                curr.system.total_dogs,
            ),
            severity: "warning",
        });
    }

    const RETIRED_SOURCES: &[&str] = &[
        "hermes-hermes",
        "hermes-x-proxy",
        "hermes-x-organ",
        "x-proxy",
        "hermes-x",
        "hermes",
        "hermes-telegram-personal",
    ];
    let now = chrono::Utc::now();
    for organ in &curr.organs {
        if !is_permanent_source(&organ.source) {
            continue;
        }
        if RETIRED_SOURCES.contains(&organ.source.as_str()) {
            continue;
        }
        if organ.total_observations < 10 {
            continue;
        }
        let silence_secs = chrono::DateTime::parse_from_rfc3339(&organ.last_observation)
            .map(|t| (now - t.with_timezone(&chrono::Utc)).num_seconds().max(0) as u64)
            .unwrap_or(organ.silence_secs);
        if silence_secs > 3600 {
            let hours = silence_secs / 3600;
            alerts.push(Alert {
                kind: "organ_silence",
                message: format!(
                    "Organ '{}' silent for {}h (last obs: {}, total: {})",
                    organ.source, hours, organ.last_observation, organ.total_observations,
                ),
                severity: if silence_secs > 86400 {
                    "critical"
                } else {
                    "warning"
                },
            });
        }
    }

    alerts
}
