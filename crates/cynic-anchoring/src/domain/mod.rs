//! Pure domain types for personal anchoring.
//!
//! No I/O. All types are serializable and testable.

use serde::{Deserialize, Serialize};

/// The domains of personal anchoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorDomain {
    Sport,
    Meditation,
    Nutrition,
    Sobriety,
    ProjectMaterialized,
    GeneralReturn,
}

/// Substances tracked for sobriety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SobrietySubstance {
    Alcohol,
    Tobacco,
    Cannabis,
    HardDrug,
}

/// Sobriety milestones in days since last use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SobrietyMilestone {
    Days1,
    Days3,
    Days7,
    Month1,
    Month3,
    Month6,
}

impl SobrietyMilestone {
    /// Returns the number of days for this milestone.
    pub fn days(&self) -> u32 {
        match self {
            SobrietyMilestone::Days1 => 1,
            SobrietyMilestone::Days3 => 3,
            SobrietyMilestone::Days7 => 7,
            SobrietyMilestone::Month1 => 30,
            SobrietyMilestone::Month3 => 90,
            SobrietyMilestone::Month6 => 180,
        }
    }

    /// Compute the highest milestone reached given days sober.
    pub fn highest_reached(days_sober: u32) -> Option<Self> {
        let milestones = [
            SobrietyMilestone::Days1,
            SobrietyMilestone::Days3,
            SobrietyMilestone::Days7,
            SobrietyMilestone::Month1,
            SobrietyMilestone::Month3,
            SobrietyMilestone::Month6,
        ];
        milestones
            .iter()
            .rev()
            .find(|m| days_sober >= m.days())
            .copied()
    }

    /// Returns all milestones reached up to the given days sober.
    pub fn all_reached(days_sober: u32) -> Vec<Self> {
        let milestones = [
            SobrietyMilestone::Days1,
            SobrietyMilestone::Days3,
            SobrietyMilestone::Days7,
            SobrietyMilestone::Month1,
            SobrietyMilestone::Month3,
            SobrietyMilestone::Month6,
        ];
        milestones
            .iter()
            .filter(|m| days_sober >= m.days())
            .copied()
            .collect()
    }
}

/// A single anchoring event (sport session, meditation, meal, sobriety check, project milestone).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnchorEvent {
    pub id: Option<u64>,
    pub domain: AnchorDomain,
    pub event_type: String,
    pub timestamp_ms: i64,
    pub duration_ms: Option<i64>,
    pub value: Option<f64>,
    pub context_json: String,
    pub sync_eligible: bool,
}

impl AnchorEvent {
    pub fn new(
        domain: AnchorDomain,
        event_type: impl Into<String>,
        timestamp_ms: i64,
    ) -> Self {
        Self {
            id: None,
            domain,
            event_type: event_type.into(),
            timestamp_ms,
            duration_ms: None,
            value: None,
            context_json: "{}".to_string(),
            sync_eligible: true,
        }
    }

    pub fn with_duration(mut self, ms: i64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    pub fn with_value(mut self, v: f64) -> Self {
        self.value = Some(v);
        self
    }

    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context_json = ctx.into();
        self
    }

    pub fn with_sync(mut self, eligible: bool) -> Self {
        self.sync_eligible = eligible;
        self
    }
}

/// A decision→result outcome pair for the outcome loop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnchorOutcome {
    pub decision_id: String,
    pub domain: AnchorDomain,
    pub result: String,
    pub measured_at_ms: i64,
    pub notes: Option<String>,
}

impl AnchorOutcome {
    pub fn new(
        decision_id: impl Into<String>,
        domain: AnchorDomain,
        result: impl Into<String>,
        measured_at_ms: i64,
    ) -> Self {
        Self {
            decision_id: decision_id.into(),
            domain,
            result: result.into(),
            measured_at_ms,
            notes: None,
        }
    }
}

/// General return at instant T: computed from events in a time window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        events: &[AnchorEvent],
        window_start_ms: i64,
        window_end_ms: i64,
    ) -> Self {
        let mut sport_minutes: u32 = 0;
        let mut meditation_minutes: u32 = 0;
        let mut nutrition_count: u32 = 0;
        let mut projects: u32 = 0;
        let mut total: u32 = 0;

        for ev in events {
            if ev.timestamp_ms < window_start_ms || ev.timestamp_ms > window_end_ms {
                continue;
            }
            total += 1;
            match ev.domain {
                AnchorDomain::Sport => {
                    sport_minutes += (ev.duration_ms.unwrap_or(0) / 60_000) as u32;
                }
                AnchorDomain::Meditation => {
                    meditation_minutes += (ev.duration_ms.unwrap_or(0) / 60_000) as u32;
                }
                AnchorDomain::Nutrition => {
                    nutrition_count += 1;
                }
                AnchorDomain::ProjectMaterialized => {
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

    #[test]
    fn milestone_highest_reached() {
        assert_eq!(SobrietyMilestone::highest_reached(0), None);
        assert_eq!(
            SobrietyMilestone::highest_reached(1),
            Some(SobrietyMilestone::Days1)
        );
        assert_eq!(
            SobrietyMilestone::highest_reached(90),
            Some(SobrietyMilestone::Month3)
        );
        assert_eq!(
            SobrietyMilestone::highest_reached(200),
            Some(SobrietyMilestone::Month6)
        );
    }

    #[test]
    fn milestone_all_reached() {
        let reached = SobrietyMilestone::all_reached(7);
        assert_eq!(reached.len(), 3);
    }

    #[test]
    fn general_return_compute() {
        let events = vec![
            AnchorEvent::new(AnchorDomain::Sport, "run", 1000).with_duration(1_800_000),
            AnchorEvent::new(AnchorDomain::Meditation, "sit", 2000).with_duration(600_000),
            AnchorEvent::new(AnchorDomain::Nutrition, "meal", 3000),
            AnchorEvent::new(AnchorDomain::ProjectMaterialized, "release", 4000),
        ];
        let gr = GeneralReturn::compute(&events, 0, 5000);
        assert_eq!(gr.sport_minutes, 30);
        assert_eq!(gr.meditation_minutes, 10);
        assert_eq!(gr.nutrition_score, Some(1.0));
        assert_eq!(gr.projects_materialized, 1);
        assert_eq!(gr.total_events, 4);
    }

    #[test]
    fn anchor_event_builder() {
        let ev = AnchorEvent::new(AnchorDomain::Sport, "run", 1000)
            .with_duration(3_600_000)
            .with_value(5.0)
            .with_context(r#"{"intensity":"high"}"#);
        assert_eq!(ev.domain, AnchorDomain::Sport);
        assert_eq!(ev.duration_ms, Some(3_600_000));
        assert_eq!(ev.value, Some(5.0));
        assert!(ev.sync_eligible);
    }
}