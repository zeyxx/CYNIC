use serde::{Deserialize, Serialize};

pub const AUDIT_REPORT_SCHEMA_V2: &str = "cynic.audit_report.v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Critical,
    Warning,
    Debt,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    Syntax,
    Boundary,
    MutationContract,
    PathSanitation,
    SchemaCompliance,
    HotspotThreshold,
    ResidualUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditSpan {
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditViolation {
    pub code: String,
    pub message: String,
    pub severity: AuditSeverity,
    pub category: AuditCategory,
    pub span: Option<AuditSpan>,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditRuleSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: AuditCategory,
    pub severity: AuditSeverity,
    pub automated_remediation_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditSummary {
    pub status: String,
    pub total_rules_evaluated: usize,
    pub passed_rules_count: usize,
    pub critical_count: usize,
    pub warning_count: usize,
    pub debt_count: usize,
    pub residual_unknown_count: usize,
    pub maturity_score_percent: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditReportV2 {
    /// Owned, not `&'static str`: a report that serializes but cannot round-trip
    /// back is not verifiable evidence. Borrowing here forces any deserializing
    /// caller into a `'static` lifetime it cannot satisfy.
    pub schema: String,
    pub generated_at: String,
    pub summary: AuditSummary,
    pub violations: Vec<AuditViolation>,
    pub evaluated_rules: Vec<AuditRuleSpec>,
    pub residual_unknowns: Vec<String>,
}

pub trait BoundedMutationContract {
    type DesiredState;
    type ObservedState;
    type AppliedEvidence;
    type RollbackPlan;

    fn preflight(&self) -> Result<Self::DesiredState, String>;
    fn observe(&self) -> Result<Self::ObservedState, String>;
    fn apply(&self, desired: &Self::DesiredState) -> Result<Self::AppliedEvidence, String>;
    fn read_back_proof(&self, evidence: &Self::AppliedEvidence) -> Result<bool, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_report_v2_serde_roundtrip() {
        let report = AuditReportV2 {
            schema: AUDIT_REPORT_SCHEMA_V2.to_string(),
            generated_at: "2026-08-14T00:00:00Z".to_string(),
            summary: AuditSummary {
                status: "ok".to_string(),
                total_rules_evaluated: 5,
                passed_rules_count: 5,
                critical_count: 0,
                warning_count: 0,
                debt_count: 0,
                residual_unknown_count: 0,
                maturity_score_percent: 100,
            },
            violations: vec![AuditViolation {
                code: "TEST-001".to_string(),
                message: "Test violation".to_string(),
                severity: AuditSeverity::Warning,
                category: AuditCategory::Syntax,
                span: Some(AuditSpan {
                    file_path: "src/lib.rs".to_string(),
                    line_start: 1,
                    line_end: 5,
                    snippet: Some("use test;".to_string()),
                }),
                remediation: Some("Remove test import".to_string()),
            }],
            evaluated_rules: vec![AuditRuleSpec {
                id: "TEST-RULE".to_string(),
                name: "test_rule".to_string(),
                description: "Test rule description".to_string(),
                category: AuditCategory::Syntax,
                severity: AuditSeverity::Warning,
                automated_remediation_available: true,
            }],
            residual_unknowns: Vec::new(),
        };

        let serialized = serde_json::to_string(&report).expect("serialize");
        let deserialized: AuditReportV2 = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(report, deserialized);
    }
}
