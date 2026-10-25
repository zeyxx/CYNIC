//! General return at instant T: computed from observations in a time window.
//!
//! Pure — no I/O.

use cynic_protocol::observation_record::Observation;

use crate::domain::observation::AnchorDomain;
use crate::domain::observation::duration_ms_of;
use crate::domain::observation::is_anchor_observation;

#[derive(Debug, Clone, PartialEq)]
pub struct GeneralReturn {
    pub computed_at_ms: i64,
    pub window_start_ms: i64,
    pub window_end_ms: i64,
    pub sport_minutes: u32,
    pub meditation_minutes: u32,
    pub nutrition_score: Option<f64>,
    pub sobriety_days: Option<u32>,
    pub projects_materialized: u32,
    pub total_events: u32,
}

impl GeneralReturn {
    pub fn compute(
        observations: &[Observation],
        window_start_ms: i64,
        window_end_ms: i64,
    ) -> Self {
        let mut sport_minutes: u32 = 0;
        let mut meditation_minutes: u32 = 0;
        let mut nutrition_count: u32 = 0;
        let mut projects: u32 = 0;
        let mut total: u32 = 0;

        for obs in observations {
            if !is_anchor_observation(obs) {
                continue;
            }
            if obs.timestamp < window_start_ms || obs.timestamp > window_end_ms {
                continue;
            }
            total += 1;
            match AnchorDomain::from_tool(&obs.tool) {
                Some(AnchorDomain::Sport) => {
                    let ms = duration_ms_of(obs).unwrap_or(0);
                    sport_minutes += (ms / 60_000) as u32;
                }
                Some(AnchorDomain::Meditation) => {
                    let ms = duration_ms_of(obs).unwrap_or(0);
                    meditation_minutes += (ms / 60_000) as u32;
                }
                Some(AnchorDomain::Nutrition) => {
                    nutrition_count += 1;
                }
                Some(AnchorDomain::ProjectMaterialized) => {
                    projects += 1;
                }
                _ => {}
            }
        }

        let nutrition_score = if nutrition_count > 0 {
            Some(nutrition_count as f64)
        } else {
            None
        };

        Self {
            computed_at_ms: window_end_ms,
            window_start_ms,
            window_end_ms,
            sport_minutes,
            meditation_minutes,
            nutrition_score,
            sobriety_days: None,
            projects_materialized: projects,
            total_events: total,
        }
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
        assert_eq!(gr.sport_minutes, 30);
        assert_eq!(gr.meditation_minutes, 10);
        assert_eq!(gr.nutrition_score, Some(1.0));
        assert_eq!(gr.projects_materialized, 1);
        assert_eq!(gr.total_events, 4);
    }
}