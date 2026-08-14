//! Routing domain values for backend and model selection.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferenceClass {
    General,
    Summarization,
    Embedding,
    ToolUse,
    JsonStrict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingConstraint {
    SovereignOnly,
    ToolUseRequired,
    JsonModeRequired,
    ModelName(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingIntent {
    pub class: InferenceClass,
    pub constraints: Vec<RoutingConstraint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingDecision {
    pub backend_name: String,
    pub reason: String,
}
