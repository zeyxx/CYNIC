//! Types for the autoresearch surface.
//!
//! Pure shapes only — no I/O here. The runner produces these, the render
//! layer formats them.

use cynic_protocol::judgment::CynicJudgment;

/// Summary of an autoresearch run (pure shape for projection).
#[derive(Debug, Clone, serde::Serialize)]
pub struct AutoresearchSummary {
    pub corpus_sessions: usize,
    pub hypotheses_evaluated: usize,
    pub kept: usize,
    pub discarded: usize,
    pub unknown: usize,
}

/// One entry in the report (pure shape).
#[derive(Debug, Clone, serde::Serialize)]
pub struct AutoresearchEntry {
    pub skill_id: String,
    pub domain: String,
    pub gain: f64,
    pub status: String,
}

/// The autoresearch surface report. Includes the optional judgment
/// projection for operator inspection.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AutoresearchReport {
    pub schema: String,
    pub summary: AutoresearchSummary,
    pub entries: Vec<AutoresearchEntry>,
    pub judgment: Option<CynicJudgment>,
}

impl AutoresearchReport {
    pub fn new(summary: AutoresearchSummary, entries: Vec<AutoresearchEntry>, judgment: Option<CynicJudgment>) -> Self {
        Self {
            schema: "cynic.autoresearch_surface.report.v1".to_string(),
            summary,
            entries,
            judgment,
        }
    }
}
