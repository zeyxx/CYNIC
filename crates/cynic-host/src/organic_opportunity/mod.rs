//! Organic opportunity observation vertical.
//!
//! Provides host observation adapters across session traces, repository worktrees,
//! and audit findings, composing them into a contestable, ranked operator compass report.

pub mod audit_observer;
pub mod port;
pub mod session_observer;
pub mod worktree_observer;

pub use audit_observer::{AuditObserverConfig, AuditValidationObserver, DebtFindingEntry};
pub use port::{OrganicOpportunityObserver, SourceObservationStatus};
pub use session_observer::{SessionObserverConfig, SessionPatternObserver};
pub use worktree_observer::{SourceWorktreeObserver, WorktreeObserverConfig};

use cynic_protocol::organic_opportunity::{
    ORGANIC_OPPORTUNITY_REPORT_SCHEMA, OpportunitySourceKind, OrganicOpportunity,
    OrganicOpportunityReport, ValueFrame, rank_in_value_frame,
};

/// Aggregator coordinating multiple opportunity observers into a unified ranked report.
pub struct OrganicCompassCollector {
    observers: Vec<Box<dyn OrganicOpportunityObserver>>,
}

impl Default for OrganicCompassCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl OrganicCompassCollector {
    pub fn new() -> Self {
        Self {
            observers: Vec::new(),
        }
    }

    pub fn with_observer(mut self, observer: Box<dyn OrganicOpportunityObserver>) -> Self {
        self.observers.push(observer);
        self
    }

    /// Run all registered observers and produce a ranked `OrganicOpportunityReport`.
    pub fn collect_report(&self, frame: &ValueFrame, now_unix_s: u64) -> OrganicOpportunityReport {
        let mut all_opportunities: Vec<OrganicOpportunity> = Vec::new();
        let mut unavailable_sources: Vec<OpportunitySourceKind> = Vec::new();

        for observer in &self.observers {
            match observer.observe() {
                SourceObservationStatus::Observed { data } => {
                    all_opportunities.extend(data);
                }
                SourceObservationStatus::Empty => {}
                SourceObservationStatus::Unavailable { kind, .. } => {
                    if !unavailable_sources.contains(&kind) {
                        unavailable_sources.push(kind);
                    }
                }
            }
        }

        let ranked = rank_in_value_frame(frame, &all_opportunities);

        OrganicOpportunityReport {
            schema: ORGANIC_OPPORTUNITY_REPORT_SCHEMA.to_string(),
            frame: frame.clone(),
            opportunities: ranked,
            unavailable_sources,
            generated_at_unix_s: now_unix_s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::organic_opportunity::{
        EnergyEstimate, ORGANIC_OPPORTUNITY_SCHEMA, ORGANIC_VALUE_FRAME_SCHEMA, OpportunityClass,
        OpportunitySource, SmallestNextStep, ValueDimension,
    };

    struct MockObserver {
        name: &'static str,
        kind: OpportunitySourceKind,
        status: SourceObservationStatus<Vec<OrganicOpportunity>>,
    }

    impl OrganicOpportunityObserver for MockObserver {
        fn observer_name(&self) -> &'static str {
            self.name
        }

        fn source_kind(&self) -> OpportunitySourceKind {
            self.kind
        }

        fn observe(&self) -> SourceObservationStatus<Vec<OrganicOpportunity>> {
            self.status.clone()
        }
    }

    fn sample_opp(id: &str, scope: &str, confidence: f64) -> OrganicOpportunity {
        OrganicOpportunity {
            schema: ORGANIC_OPPORTUNITY_SCHEMA.to_string(),
            opportunity_id: id.to_string(),
            scope: scope.to_string(),
            sources: vec![OpportunitySource {
                kind: OpportunitySourceKind::SessionTrace,
                ref_id: "test:ref".to_string(),
                label: None,
            }],
            class: OpportunityClass::Emerging,
            expected_unlock: "test unlock".to_string(),
            estimated_energy: EnergyEstimate::Unknown {
                reason: "test".to_string(),
            },
            freshness_seconds: 10,
            confidence,
            smallest_next_step: SmallestNextStep::Observe,
            residual_unknowns: vec![],
            operator_disposition: Default::default(),
        }
    }

    #[test]
    fn collector_aggregates_and_ranks_opportunities() {
        let obs1 = MockObserver {
            name: "obs1",
            kind: OpportunitySourceKind::SessionTrace,
            status: SourceObservationStatus::Observed {
                data: vec![sample_opp("opp-low", "scope/a", 0.6)],
            },
        };
        let obs2 = MockObserver {
            name: "obs2",
            kind: OpportunitySourceKind::SourceWorktree,
            status: SourceObservationStatus::Observed {
                data: vec![sample_opp("opp-high", "scope/b", 0.9)],
            },
        };
        let obs3 = MockObserver {
            name: "obs3",
            kind: OpportunitySourceKind::ValidationAudit,
            status: SourceObservationStatus::Unavailable {
                kind: OpportunitySourceKind::ValidationAudit,
                reason: "no audit file".to_string(),
            },
        };

        let frame = ValueFrame {
            schema: ORGANIC_VALUE_FRAME_SCHEMA.to_string(),
            frame_id: "frame-conf".to_string(),
            description: "high confidence first".to_string(),
            tie_break: vec![ValueDimension::ConfidenceHigh],
        };

        let collector = OrganicCompassCollector::new()
            .with_observer(Box::new(obs1))
            .with_observer(Box::new(obs2))
            .with_observer(Box::new(obs3));

        let report = collector.collect_report(&frame, 1724000000);

        assert_eq!(report.schema, ORGANIC_OPPORTUNITY_REPORT_SCHEMA);
        assert_eq!(report.opportunities.len(), 2);
        assert_eq!(report.opportunities[0].opportunity_id, "opp-high");
        assert_eq!(report.opportunities[1].opportunity_id, "opp-low");
        assert_eq!(report.unavailable_sources.len(), 1);
        assert_eq!(
            report.unavailable_sources[0],
            OpportunitySourceKind::ValidationAudit
        );
    }
}
