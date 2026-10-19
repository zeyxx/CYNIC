//! Pattern GC opportunity observer.
//!
//! Interconnects generational pattern garbage collection (`cynic_protocol::rate_classify`)
//! with the organic opportunities compass.

use std::path::PathBuf;

use cynic_protocol::organic_opportunity::{
    EnergyEstimate, ORGANIC_OPPORTUNITY_SCHEMA, OpportunityClass, OpportunitySource,
    OpportunitySourceKind, OrganicOpportunity, SmallestNextStep,
};
use cynic_protocol::rate_classify::{
    RateTrend, WindowedCount, CountWindows, Thresholds,
};

use super::port::{OrganicOpportunityObserver, SourceObservationStatus};

/// Configuration for Pattern GC opportunity observation.
#[derive(Debug, Clone)]
pub struct RateTrendObserverConfig {
    pub pattern_report_path: PathBuf,
    pub retention_policy: Thresholds,
}

impl Default for RateTrendObserverConfig {
    fn default() -> Self {
        Self {
            pattern_report_path: PathBuf::from("infra/rate_classify_report.json"),
            retention_policy: Thresholds::default(),
        }
    }
}

/// Host adapter observing generational pattern assessments.
pub struct RateTrendObserver {
    config: RateTrendObserverConfig,
}

impl RateTrendObserver {
    pub fn new(config: RateTrendObserverConfig) -> Self {
        Self { config }
    }

    /// Pure analyzer: turns classified pattern assessments into organic opportunities.
    pub fn analyze_observations(
        observations: &[WindowedCount],
        policy: &Thresholds,
        _now_unix_s: u64,
    ) -> Vec<OrganicOpportunity> {
        let mut opportunities = Vec::new();

        for observation in observations {
            let assessment = cynic_protocol::rate_classify::classify(observation, policy);
            let (class, step, unlock_prefix) = match assessment.class {
                RateTrend::Emerging => (
                    OpportunityClass::Emerging,
                    SmallestNextStep::Plan,
                    "Investigate emerging pattern",
                ),
                RateTrend::Recurring => (
                    OpportunityClass::RecurringFriction,
                    SmallestNextStep::ProposeRepair,
                    "Distill & repair recurring pattern",
                ),
                RateTrend::Fading => (
                    OpportunityClass::Plumbing,
                    SmallestNextStep::Consolidate,
                    "Consolidate fading pattern into durable artifact",
                ),
                RateTrend::Unknown => continue,
            };

            let digest = cynic_protocol::digest::digest_json(&serde_json::json!({
                "key": assessment.key,
                "class": assessment.class,
                "reasons": assessment.reasons,
            }))
            .unwrap_or_else(|_| format!("sha3-512:{}", assessment.key));

            let confidence = match assessment.class {
                RateTrend::Recurring => 0.90,
                RateTrend::Emerging => 0.75,
                RateTrend::Fading => 0.80,
                RateTrend::Unknown => 0.50,
            };

            let opp = OrganicOpportunity {
                schema: ORGANIC_OPPORTUNITY_SCHEMA.to_string(),
                opportunity_id: format!("opp:rate_classify:{}", &digest[..16.min(digest.len())]),
                scope: format!("pattern/{}", assessment.key),
                sources: vec![OpportunitySource {
                    kind: OpportunitySourceKind::RateTrend,
                    ref_id: format!("pattern_key:{}", assessment.key),
                    label: Some(format!("class: {:?}", assessment.class)),
                }],
                class,
                expected_unlock: format!("{} '{}'", unlock_prefix, assessment.key),
                estimated_energy: EnergyEstimate::Range {
                    method: "pattern distillation / repair".to_string(),
                    low_joules: 30.0,
                    high_joules: 300.0,
                },
                freshness_seconds: 0,
                confidence,
                smallest_next_step: step,
                residual_unknowns: assessment.reasons.clone(),
                operator_disposition: Default::default(),
            };

            opportunities.push(opp);
        }

        opportunities
    }
}

impl OrganicOpportunityObserver for RateTrendObserver {
    fn observer_name(&self) -> &'static str {
        "rate_classify_observer"
    }

    fn source_kind(&self) -> OpportunitySourceKind {
        OpportunitySourceKind::RateTrend
    }

    fn observe(&self) -> SourceObservationStatus<Vec<OrganicOpportunity>> {
        let path = &self.config.pattern_report_path;
        if !path.exists() {
            return SourceObservationStatus::Unavailable {
                kind: OpportunitySourceKind::RateTrend,
                reason: format!("pattern report file not found: {}", path.display()),
            };
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return SourceObservationStatus::Unavailable {
                    kind: OpportunitySourceKind::RateTrend,
                    reason: format!("failed to read pattern report {}: {}", path.display(), e),
                };
            }
        };

        let observations: Vec<WindowedCount> = match serde_json::from_str(&content) {
            Ok(obs) => obs,
            Err(e) => {
                return SourceObservationStatus::Unavailable {
                    kind: OpportunitySourceKind::RateTrend,
                    reason: format!("failed to parse pattern report JSON: {}", e),
                };
            }
        };

        if observations.is_empty() {
            return SourceObservationStatus::Empty;
        }

        let opps = Self::analyze_observations(&observations, &self.config.retention_policy, 0);

        if opps.is_empty() {
            SourceObservationStatus::Empty
        } else {
            SourceObservationStatus::Observed { data: opps }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emerging_pattern_generates_emerging_opportunity() {
        let policy = Thresholds {
            recurring_threshold_abs: 100,
            emergence_floor_abs: 10,
            emergence_ratio_epsilon: 0.1,
            fading_ratio_ceiling: 0.2,
        };

        let obs = WindowedCount {
            key: "toolchain_ice_new_variant".to_string(),
            windows: CountWindows {
                historical: 2,
                recent: 20,
                historical_span: 10,
                recent_span: 2,
            },
        };

        let opps = RateTrendObserver::analyze_observations(&[obs], &policy, 0);
        assert_eq!(opps.len(), 1);
        assert_eq!(opps[0].class, OpportunityClass::Emerging);
        assert_eq!(opps[0].smallest_next_step, SmallestNextStep::Plan);
        assert_eq!(opps[0].scope, "pattern/toolchain_ice_new_variant");
        assert_eq!(opps[0].sources[0].kind, OpportunitySourceKind::RateTrend);
    }

    #[test]
    fn recurring_pattern_generates_repair_opportunity() {
        let policy = Thresholds {
            recurring_threshold_abs: 50,
            emergence_floor_abs: 10,
            emergence_ratio_epsilon: 0.1,
            fading_ratio_ceiling: 0.2,
        };

        let obs = WindowedCount {
            key: "docker_socket_stale".to_string(),
            windows: CountWindows::comparable(80, 75),
        };

        let opps = RateTrendObserver::analyze_observations(&[obs], &policy, 0);
        assert_eq!(opps.len(), 1);
        assert_eq!(opps[0].class, OpportunityClass::RecurringFriction);
        assert_eq!(opps[0].smallest_next_step, SmallestNextStep::ProposeRepair);
    }
}
