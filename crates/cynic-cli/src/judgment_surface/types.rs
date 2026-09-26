//! Types for the judgment surface.
//!
//! Pure shapes only — no I/O here. The scan layer produces these, the render
//! layer formats them.

use cynic_eval::judgment::CynicJudgment;

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
