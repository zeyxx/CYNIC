use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisStage {
    Heuristic,
    ClassicalMl,
    LocalModel,
    LocalMultimodal,
}

impl AnalysisStage {
    pub const ORDERED: [Self; 4] = [
        Self::Heuristic,
        Self::ClassicalMl,
        Self::LocalModel,
        Self::LocalMultimodal,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisDisposition {
    LikelyWithinPolicy,
    LikelyOutsidePolicy,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentAnalysisPolicy {
    pub target_confidence_bps: u16,
    pub maximum_stage: AnalysisStage,
    pub require_calibrated_result: bool,
    pub stop_on_material_disagreement: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentAnalysisResult {
    pub stage: AnalysisStage,
    pub disposition: AnalysisDisposition,
    pub confidence_bps: u16,
    pub calibrated: bool,
    pub material_disagreement: bool,
    pub labels: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentCascadeDecision {
    pub status: String,
    pub accepted_stage: Option<AnalysisStage>,
    pub next_stage: Option<AnalysisStage>,
    pub disposition: AnalysisDisposition,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
}
