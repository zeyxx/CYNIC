//! General return at instant T: computed from observations in a time window.
//!
//! Pure — no I/O. Metrics are stored in a BTreeMap so that divergent
//! profiles can add their own named metrics without modifying this code.

use std::collections::BTreeMap;

use cynic_protocol::observation_record::Observation;

use crate::domain::observation::is_anchor_observation;
use crate::domain::observation::duration_ms_of;

/// Standard metric keys.
pub const METRIC_SPORT_MINUTES: &str = "sport_minutes";
pub const METRIC_MEDITATION_MINUTES: &str = "meditation_minutes";
pub const METRIC_NUTRITION_COUNT: &str = "nutrition_count";
pub const METRIC_PROJECTS_MATERIALIZED: &str = "projects_materialized";
pub const METRIC_TOTAL_EVENTS: &str = "total_events";

#[derive(Debug, Clone, PartialEq)]
pub struct GeneralReturn {
    pub computed_at_ms: i64,
    pub window_start_ms: i64,
    pub window_end_ms: i64,
    /// Named metrics. Standard keys are defined as METRIC_* constants.
    /// Divergent profiles add their own keys (e.g. "stimming_minutes",
    /// "routine_adherence") without modifying this struct.
    pub metrics: BTreeMap<String, f64>,
}

impl GeneralReturn {
    pub fn compute(observations: &[Observation], window_start_ms: i64, window_end_ms: i64) -> Self {
        let mut metrics: BTreeMap<String, f64> = BTreeMap::new();
        metrics.insert(METRIC_SPORT_MINUTES.to_string(), 0.0);
        metrics.insert(METRIC_MEDITATION_MINUTES.to_string(), 0.0);
        metrics.insert(METRIC_NUTRITION_COUNT.to_string(), 0.0);
        metrics.insert(METRIC_PROJECTS_MATERIALIZED.to_string(), 0.0);
        metrics.insert(METRIC_TOTAL_EVENTS.to_string(), 0.0);

        for obs in observations {
            if !is_anchor_observation(obs) {
                continue;
            }
            if obs.timestamp < window_start_ms || obs.timestamp > window_end_ms {
                continue;
            }
            *metrics.get_mut(METRIC_TOTAL_EVENTS).unwrap() += 1.0;

            match obs.tool.as_str() {
                "sport" => {
                    let ms = duration_ms_of(obs).unwrap_or(0);
                    *metrics.get_mut(METRIC_SPORT_MINUTES).unwrap() += (ms / 60_000) as f64;
                }
                "meditation" => {
                    let ms = duration_ms_of(obs).unwrap_or(0);
                    *metrics.get_mut(METRIC_MEDITATION_MINUTES).unwrap() += (ms / 60_000) as f64;
                }
                "nutrition" => {
                    *metrics.get_mut(METRIC_NUTRITION_COUNT).unwrap() += 1.0;
                }
                "project_materialized" => {
                    *metrics.get_mut(METRIC_PROJECTS_MATERIALIZED).unwrap() += 1.0;
                }
                // Divergent profiles: unknown tools counted in total_events
                // and their numeric value stored under the tool name.
                other => {
                    if let Some(v) = obs.value.as_ref().and_then(|v| v.as_f64()) {
                        let entry = metrics.entry(other.to_string()).or_insert(0.0);
                        *entry += v;
                    }
                }
            }
        }

        Self {
            computed_at_ms: window_end_ms,
            window_start_ms,
            window_end_ms,
            metrics,
        }
    }

    /// Convenience accessor for sport minutes.
    pub fn sport_minutes(&self) -> u32 {
        self.metrics.get(METRIC_SPORT_MINUTES).copied().unwrap_or(0.0) as u32
    }

    /// Convenience accessor for meditation minutes.
    pub fn meditation_minutes(&self) -> u32 {
        self.metrics.get(METRIC_MEDITATION_MINUTES).copied().unwrap_or(0.0) as u32
    }

    /// Convenience accessor for nutrition count.
    pub fn nutrition_count(&self) -> u32 {
        self.metrics.get(METRIC_NUTRITION_COUNT).copied().unwrap_or(0.0) as u32
    }

    /// Convenience accessor for projects materialized.
    pub fn projects_materialized(&self) -> u32 {
        self.metrics.get(METRIC_PROJECTS_MATERIALIZED).copied().unwrap_or(0.0) as u32
    }

    /// Convenience accessor for total events.
    pub fn total_events(&self) -> u32 {
        self.metrics.get(METRIC_TOTAL_EVENTS).copied().unwrap_or(0.0) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::observation_record::Observation;

    fn make_obs(tool: &str, timestamp: i64, duration_ms: Option<i64>) -> Observation {
        Observation {
            project: "CYNIC".to_string(),
            agent_id: "kyon-device-local".to_string(),
            tool: tool.to_string(),
            target: "test".to_string(),
            domain: "personal-anchoring".to_string(),
            status: "observed".to_string(),
            context: "{}".to_string(),
            session_id: "s1".to_string(),
            timestamp,
            tags: vec![],
            source_tier: String::new(),
            value: duration_ms.map(|d| serde_json::json!({"duration_ms": d})),
            confidence: None,
            consumer: None,
            action: None,
            depends_on: vec![],
            maturity: None,
            hash: String::new(),
            prev_hash: String::new(),
            observers: vec![],
            consensus_score: None,
        }
    }

    #[test]
    fn general_return_compute() {
        let obs = vec![
            make_obs("sport", 1000, Some(1_800_000)),
            make_obs("meditation", 2000, Some(600_000)),
            make_obs("nutrition", 3000, None),
            make_obs("project_materialized", 4000, None),
        ];
        let gr = GeneralReturn::compute(&obs, 0, 5000);
        assert_eq!(gr.sport_minutes(), 30);
        assert_eq!(gr.meditation_minutes(), 10);
        assert_eq!(gr.nutrition_count(), 1);
        assert_eq!(gr.projects_materialized(), 1);
        assert_eq!(gr.total_events(), 4);
    }

    #[test]
    fn divergent_profile_custom_metric() {
        // A divergent profile pushes a custom tool with a numeric value.
        let mut obs = make_obs("stimming", 1000, None);
        obs.value = Some(serde_json::json!(42.0));
        let gr = GeneralReturn::compute(&[obs], 0, 5000);
        assert_eq!(gr.total_events(), 1);
        assert_eq!(gr.metrics.get("stimming"), Some(&42.0));
    }
}