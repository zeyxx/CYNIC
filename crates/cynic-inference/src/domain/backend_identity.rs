//! Backend identity and capability values.

use crate::domain::inference::BackendStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendCapability {
    Chat,
    Embedding,
    Summarization,
    ToolUse,
    JsonMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelIdentity {
    pub backend_name: String,
    pub model_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendIdentity {
    pub backend_name: String,
    pub display_name: String,
    pub model_name: Option<String>,
    pub sovereign: bool,
    pub capabilities: Vec<BackendCapability>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackendHealthSnapshot {
    pub identity: BackendIdentity,
    pub status: BackendStatus,
}
