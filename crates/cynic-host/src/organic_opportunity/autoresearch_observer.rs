//! Autoresearch opportunity observer.
//!
//! Interconnects autonomous skill research (`cynic_protocol::autoresearch`)
//! with the organic opportunities compass.

use std::path::{Path, PathBuf};

use cynic_protocol::autoresearch::{AutoresearchReport, KeepDiscard, SkillCandidate};
use cynic_protocol::organic_opportunity::{
    EnergyEstimate, ORGANIC_OPPORTUNITY_SCHEMA, OpportunityClass, OpportunitySource,
    OpportunitySourceKind, OrganicOpportunity, SmallestNextStep,
};

use super::port::{OrganicOpportunityObserver, SourceObservationStatus};

/// Configuration for Autoresearch opportunity observation.
#[derive(Debug, Clone)]
pub struct AutoresearchObserverConfig {
    pub report_path: PathBuf,
}

impl Default for AutoresearchObserverConfig {
    fn default() -> Self {
        Self {
            report_path: PathBuf::from("infra/autoresearch_report.json"),
        }
    }
}

/// Host adapter observing evaluated skill candidates from Autoresearch.
pub struct AutoresearchObserver {
    config: AutoresearchObserverConfig,
}

impl AutoresearchObserver {
    pub fn new(config: AutoresearchObserverConfig) -> Self {
        Self { config }
    }

    /// Pure analyzer: turns kept skill candidates into organic opportunities.
    pub fn analyze_candidates(
        candidates: &[SkillCandidate],
        _now_unix_s: u64,
    ) -> Vec<OrganicOpportunity> {
        let mut opportunities = Vec::new();

        for candidate in candidates {
            if candidate.decision == KeepDiscard::Keep {
                let digest = candidate.digest();

                let opp = OrganicOpportunity {
                    schema: ORGANIC_OPPORTUNITY_SCHEMA.to_string(),
                    opportunity_id: format!(
                        "opp:skill_candidate:{}",
                        &digest[..16.min(digest.len())]
                    ),
                    scope: format!("skill/{}", candidate.hypothesis.skill_id),
                    sources: vec![OpportunitySource {
                        kind: OpportunitySourceKind::Autoresearch,
                        ref_id: digest,
                        label: Some(format!(
                            "domain: {}, gain_coverage: {:.2}",
                            candidate.hypothesis.domain, candidate.gain.coverage
                        )),
                    }],
                    class: OpportunityClass::Emerging,
                    expected_unlock: format!(
                        "Crystallize emerged skill candidate '{}' for domain '{}' (coverage gain: {:.1}%)",
                        candidate.hypothesis.skill_id,
                        candidate.hypothesis.domain,
                        candidate.gain.coverage * 100.0
                    ),
                    estimated_energy: EnergyEstimate::Range {
                        method: "skill crystallization slice".to_string(),
                        low_joules: 20.0,
                        high_joules: 150.0,
                    },
                    freshness_seconds: 0,
                    confidence: (0.60 + candidate.gain.coverage * 0.35).min(0.95),
                    smallest_next_step: SmallestNextStep::ProposeRepair,
                    residual_unknowns: vec![
                        format!("evidence_refs_count: {}", candidate.evidence_refs.len()),
                        format!("actionability: {}", candidate.hypothesis.actionability),
                    ],
                    operator_disposition: Default::default(),
                };

                opportunities.push(opp);
            }
        }

        opportunities
    }

    /// Extract opportunities from an existing AutoresearchReport.
    pub fn analyze_report(report: &AutoresearchReport) -> Vec<OrganicOpportunity> {
        let mut opportunities = Vec::new();

        for entry in &report.entries {
            if entry.status == KeepDiscard::Keep {
                let digest = cynic_protocol::digest::digest_json(&serde_json::json!({
                    "skill_id": entry.skill_id,
                    "domain": entry.domain,
                    "gain": entry.gain,
                }))
                .unwrap_or_else(|_| format!("sha256:{}", entry.skill_id));

                let opp = OrganicOpportunity {
                    schema: ORGANIC_OPPORTUNITY_SCHEMA.to_string(),
                    opportunity_id: format!("opp:skill_entry:{}", &digest[..16.min(digest.len())]),
                    scope: format!("skill/{}", entry.skill_id),
                    sources: vec![OpportunitySource {
                        kind: OpportunitySourceKind::Autoresearch,
                        ref_id: format!("report_entry:{}", entry.skill_id),
                        label: Some(format!("domain: {}, gain: {:.2}", entry.domain, entry.gain)),
                    }],
                    class: OpportunityClass::Emerging,
                    expected_unlock: format!(
                        "Crystallize skill candidate '{}' (gain: {:.2}): {}",
                        entry.skill_id, entry.gain, entry.description
                    ),
                    estimated_energy: EnergyEstimate::Range {
                        method: "skill crystallization".to_string(),
                        low_joules: 25.0,
                        high_joules: 200.0,
                    },
                    freshness_seconds: 0,
                    confidence: 0.85,
                    smallest_next_step: SmallestNextStep::ProposeRepair,
                    residual_unknowns: vec![format!("domain: {}", entry.domain)],
                    operator_disposition: Default::default(),
                };

                opportunities.push(opp);
            }
        }

        opportunities
    }
}

impl OrganicOpportunityObserver for AutoresearchObserver {
    fn observer_name(&self) -> &'static str {
        "autoresearch_observer"
    }

    fn source_kind(&self) -> OpportunitySourceKind {
        OpportunitySourceKind::Autoresearch
    }

    fn observe(&self) -> SourceObservationStatus<Vec<OrganicOpportunity>> {
        let path = &self.config.report_path;
        if !path.exists() {
            return SourceObservationStatus::Unavailable {
                kind: OpportunitySourceKind::Autoresearch,
                reason: format!("autoresearch report not found: {}", path.display()),
            };
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return SourceObservationStatus::Unavailable {
                    kind: OpportunitySourceKind::Autoresearch,
                    reason: format!(
                        "failed to read autoresearch report {}: {}",
                        path.display(),
                        e
                    ),
                };
            }
        };

        let report: AutoresearchReport = match serde_json::from_str(&content) {
            Ok(r) => r,
            Err(e) => {
                return SourceObservationStatus::Unavailable {
                    kind: OpportunitySourceKind::Autoresearch,
                    reason: format!("failed to parse autoresearch report JSON: {}", e),
                };
            }
        };

        let opps = Self::analyze_report(&report);

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
    use cynic_protocol::autoresearch::AutoresearchReportEntry;

    #[test]
    fn kept_report_entry_generates_emerging_opportunity() {
        let report = AutoresearchReport {
            schema: "cynic.autoresearch.report.v1".to_string(),
            program_digest: "sha256:demo".to_string(),
            corpus_sessions: 10,
            hypotheses_evaluated: 2,
            kept: vec!["cynic-skill-a".to_string()],
            discarded: vec!["cynic-skill-b".to_string()],
            entries: vec![
                AutoresearchReportEntry {
                    skill_id: "cynic-skill-a".to_string(),
                    domain: "toolchain_ice".to_string(),
                    gain: 0.45,
                    status: KeepDiscard::Keep,
                    description: "Auto-remediation skill for toolchain ice".to_string(),
                },
                AutoresearchReportEntry {
                    skill_id: "cynic-skill-b".to_string(),
                    domain: "duplicate_domain".to_string(),
                    gain: 0.0,
                    status: KeepDiscard::Discard,
                    description: "Duplicate domain skill".to_string(),
                },
            ],
        };

        let opps = AutoresearchObserver::analyze_report(&report);
        assert_eq!(opps.len(), 1);
        assert_eq!(opps[0].scope, "skill/cynic-skill-a");
        assert_eq!(opps[0].class, OpportunityClass::Emerging);
        assert_eq!(opps[0].smallest_next_step, SmallestNextStep::ProposeRepair);
        assert_eq!(opps[0].sources[0].kind, OpportunitySourceKind::Autoresearch);
    }
}
