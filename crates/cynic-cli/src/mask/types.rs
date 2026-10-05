//! Pure data types for the mask surface.
//!
//! No I/O here — only shapes. The scan layer produces these, the render
//! layer formats them.

use cynic_eval::judgment::CynicJudgment;

/// Agent citizenship record projected through the mask.
/// Thin re-declaration so the CLI can serialize its intent; the canonical
/// trait lives in `cynic-protocol::agent_citizenship::Citoyenneté`.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct AgentCitizenship {
    pub schema: &'static str,
    pub agent_id: String,
    pub scope: String,
    pub mode: String,
    pub budgets: AgentBudgets,
    pub residual_risk: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct AgentBudgets {
    pub actions: u64,
    pub wall_seconds: u64,
}

/// The judgment surface report. Currently a single judgment with metadata
/// about how it was produced.
#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct JudgmentReport {
    pub schema: String,
    pub digest: String,
    pub judgment: CynicJudgment,
    pub observed_at_unix_s: u64,
}

impl JudgmentReport {
    pub(super) fn new(judgment: CynicJudgment, observed_at_unix_s: u64) -> Self {
        let digest = judgment.digest().unwrap_or_default();
        Self {
            schema: "cynic.judgment_surface.report.v1".to_string(),
            digest,
            judgment,
            observed_at_unix_s,
        }
    }
}
