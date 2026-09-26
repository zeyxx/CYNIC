//! Audit & validation finding opportunity observer.
//!
//! Observes debt taxonomy, crate audit, and gate findings to surface
//! high-ROI plumbing debt and structural anomalies.

use std::path::PathBuf;

use cynic_protocol::organic_opportunity::{
    EnergyEstimate, ORGANIC_OPPORTUNITY_SCHEMA, OpportunityClass, OpportunitySource,
    OpportunitySourceKind, OrganicOpportunity, SmallestNextStep,
};
use serde::{Deserialize, Serialize};

use super::port::{OrganicOpportunityObserver, SourceObservationStatus};

/// Summary entry of a detected debt finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebtFindingEntry {
    pub debt_id: String,
    pub nature: String,
    pub file_line: Option<String>,
    pub tier: String,         // "tier1", "tier2", "partial"
    pub blast_radius: String, // "1", "N", "infinity"
    pub unlock_description: String,
}

/// Configuration for audit finding observation.
#[derive(Debug, Clone)]
pub struct AuditObserverConfig {
    /// Injected path to debt taxonomy or audit findings JSON/Markdown.
    pub debt_file_path: PathBuf,
}

impl Default for AuditObserverConfig {
    fn default() -> Self {
        Self {
            debt_file_path: PathBuf::from("infra/DEBT_TAXONOMY.md"),
        }
    }
}

/// Host adapter observing audit findings and debt tiers.
pub struct AuditValidationObserver {
    config: AuditObserverConfig,
}

impl AuditValidationObserver {
    pub fn new(config: AuditObserverConfig) -> Self {
        Self { config }
    }

    /// Pure analyzer: turns structured debt findings into opportunities.
    pub fn analyze_findings(
        findings: &[DebtFindingEntry],
        _now_unix_s: u64,
    ) -> Vec<OrganicOpportunity> {
        let mut opportunities = Vec::new();

        for finding in findings {
            let is_tier1 = finding.tier.eq_ignore_ascii_case("tier1");
            let is_high_blast = finding.blast_radius == "infinity" || finding.blast_radius == "N";

            let class = if finding.nature.contains("plumbing") || is_tier1 {
                OpportunityClass::Plumbing
            } else {
                OpportunityClass::Dependency
            };

            let digest = cynic_node_protocol::digest::digest_json(&serde_json::json!({
                "debt_id": finding.debt_id,
                "file_line": finding.file_line,
            }))
            .unwrap_or_else(|_| format!("sha256:{}", finding.debt_id));

            let confidence = if is_tier1 { 0.90 } else { 0.70 };

            let opp = OrganicOpportunity {
                schema: ORGANIC_OPPORTUNITY_SCHEMA.to_string(),
                opportunity_id: format!("opp:audit_debt:{}", &digest[..16.min(digest.len())]),
                scope: format!("debt/{}", finding.debt_id),
                sources: vec![OpportunitySource {
                    kind: OpportunitySourceKind::ValidationAudit,
                    ref_id: format!("debt_entry:{}", finding.debt_id),
                    label: finding.file_line.clone(),
                }],
                class,
                expected_unlock: format!(
                    "Purge {} debt '{}': {}",
                    finding.tier, finding.debt_id, finding.unlock_description
                ),
                estimated_energy: EnergyEstimate::Range {
                    method: "debt purge slice".to_string(),
                    low_joules: if is_high_blast { 100.0 } else { 20.0 },
                    high_joules: if is_high_blast { 2000.0 } else { 300.0 },
                },
                freshness_seconds: 0,
                confidence,
                smallest_next_step: if is_tier1 {
                    SmallestNextStep::ProposeRepair
                } else {
                    SmallestNextStep::Plan
                },
                residual_unknowns: vec![
                    format!("blast_radius: {}", finding.blast_radius),
                    format!("tier: {}", finding.tier),
                ],
                operator_disposition: Default::default(),
            };

            opportunities.push(opp);
        }

        opportunities
    }

    /// Extract debt finding entries from a debt taxonomy Markdown file.
    pub fn parse_debt_markdown(content: &str) -> Vec<DebtFindingEntry> {
        let mut entries = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            // Look for lines matching: "### <ID> — <NATURE>" or "- <ID> <DESCRIPTION>"
            if trimmed.starts_with("### ") {
                let parts: Vec<&str> = trimmed[4..].splitn(2, " — ").collect();
                if parts.len() == 2 {
                    let debt_id = parts[0].trim().to_string();
                    let nature = parts[1].trim().to_string();
                    entries.push(DebtFindingEntry {
                        debt_id: debt_id.clone(),
                        nature,
                        file_line: None,
                        tier: "tier1".to_string(),
                        blast_radius: "N".to_string(),
                        unlock_description: format!("Resolve architectural debt {}", debt_id),
                    });
                }
            }
        }

        entries
    }
}

impl OrganicOpportunityObserver for AuditValidationObserver {
    fn observer_name(&self) -> &'static str {
        "audit_validation_observer"
    }

    fn source_kind(&self) -> OpportunitySourceKind {
        OpportunitySourceKind::ValidationAudit
    }

    fn observe(&self) -> SourceObservationStatus<Vec<OrganicOpportunity>> {
        let path = &self.config.debt_file_path;
        if !path.exists() {
            return SourceObservationStatus::Unavailable {
                kind: OpportunitySourceKind::ValidationAudit,
                reason: format!("debt findings file does not exist: {}", path.display()),
            };
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return SourceObservationStatus::Unavailable {
                    kind: OpportunitySourceKind::ValidationAudit,
                    reason: format!("failed to read debt findings {}: {}", path.display(), e),
                };
            }
        };

        let findings = Self::parse_debt_markdown(&content);
        if findings.is_empty() {
            return SourceObservationStatus::Empty;
        }

        let opps = Self::analyze_findings(&findings, 0);
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
    fn parses_debt_taxonomy_markdown_headers() {
        let sample = "\
# DEBT_TAXONOMY

### Sec1 — Capability bounded
Some description

### Layer4 — Host 4 Layers
4 layers not owned
";
        let findings = AuditValidationObserver::parse_debt_markdown(sample);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].debt_id, "Sec1");
        assert_eq!(findings[1].debt_id, "Layer4");
    }

    #[test]
    fn tier1_debt_creates_plumbing_opportunity_with_repair_step() {
        let findings = vec![DebtFindingEntry {
            debt_id: "Sec1".to_string(),
            nature: "plumbing".to_string(),
            file_line: Some("crates/cynic-protocol/src/one_shot.rs:12".to_string()),
            tier: "tier1".to_string(),
            blast_radius: "infinity".to_string(),
            unlock_description: "bounded capability issuance".to_string(),
        }];

        let opps = AuditValidationObserver::analyze_findings(&findings, 0);
        assert_eq!(opps.len(), 1);
        assert_eq!(opps[0].class, OpportunityClass::Plumbing);
        assert_eq!(opps[0].smallest_next_step, SmallestNextStep::ProposeRepair);
        assert_eq!(opps[0].scope, "debt/Sec1");
    }

    #[test]
    fn unavailable_when_debt_file_missing() {
        let observer = AuditValidationObserver::new(AuditObserverConfig {
            debt_file_path: PathBuf::from("/nonexistent/debt/path/999.md"),
        });
        let status = observer.observe();
        assert!(status.is_unavailable());
    }
}
