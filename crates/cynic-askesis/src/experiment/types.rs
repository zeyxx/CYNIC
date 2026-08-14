use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomainSpec {
    pub id: String,
    pub task_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DimensionSpec {
    pub id: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScaleSpec {
    pub id: String,
    pub sample_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExperimentSpec {
    pub schema: String,
    pub id: String,
    pub domains: Vec<DomainSpec>,
    pub dimensions: Vec<DimensionSpec>,
    pub scales: Vec<ScaleSpec>,
    pub repetitions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrialPlan {
    pub schema: &'static str,
    pub experiment_id: String,
    pub trial_id: String,
    pub domain: DomainSpec,
    pub coordinates: Vec<(String, String)>,
    pub scale: ScaleSpec,
    pub repetition: u32,
    pub mutates: bool,
    pub status: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LetterMark {
    Absent,
    Present,
    Correct,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WordleObservation {
    pub guess: String,
    pub marks: Vec<LetterMark>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WordleStepProof {
    pub step: u32,
    pub desired_action: String,
    pub observation: WordleObservation,
    pub remaining_candidates: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WordleEpisodeProof {
    pub schema: &'static str,
    pub status: &'static str,
    pub policy: String,
    pub word_length: usize,
    pub max_steps: u32,
    pub solved: bool,
    pub steps: Vec<WordleStepProof>,
    pub unknowns: Vec<String>,
}

