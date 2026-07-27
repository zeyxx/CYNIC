use serde::{Deserialize, Serialize};

use super::AnalysisStage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputExposure {
    EnvelopeOnly,
    DerivedFeatures,
    LocalRaw,
    LocalDecoded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisLocality {
    OnDevice,
    SovereignNode,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisModality {
    Text,
    Image,
    Audio,
    Video,
    BehavioralEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisCapability {
    pub id: String,
    pub stage: AnalysisStage,
    pub modalities: Vec<AnalysisModality>,
    pub input_exposure: InputExposure,
    pub locality: AnalysisLocality,
    pub estimated_cost_units: u32,
    pub estimated_latency_ms: u32,
    pub calibration_ref: Option<String>,
    pub evidence_kinds: Vec<String>,
    pub available: bool,
    pub availability_evidence_refs: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentAnalysisRequest {
    pub content_ref: String,
    pub modality: AnalysisModality,
    pub stage: AnalysisStage,
    pub policy_version: String,
    pub purpose: String,
    pub maximum_input_exposure: InputExposure,
}
