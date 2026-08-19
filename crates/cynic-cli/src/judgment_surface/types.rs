//! Types for the judgment surface.
//!
//! Pure shapes only — no I/O here. The scan layer produces these, the render
//! layer formats them.

use cynic_protocol::CynicJudgment;

/// The judgment surface report. Currently a single judgment with metadata
/// about how it was produced.
#[derive(Debug, Clone, serde::Serialize)]
pub struct JudgmentReport {
    pub schema: String,
    pub judgment: CynicJudgment,
    pub observed_at_unix_s: u64,
}

impl JudgmentReport {
    pub fn new(judgment: CynicJudgment, observed_at_unix_s: u64) -> Self {
        Self {
            schema: "cynic.judgment_surface.report.v1".to_string(),
            judgment,
            observed_at_unix_s,
        }
    }
}
