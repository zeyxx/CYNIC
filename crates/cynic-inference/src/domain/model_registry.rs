//! Model registry domain types.
//!
//! This is inference-owned law: backend identity, model capabilities, health,
//! and sovereignty belong to the model runtime, not to kernel coordination
//! storage.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceEngine {
    Ollama,
    LlamaServer,
    OpenAi,
    Other(String),
}

impl std::fmt::Display for InferenceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InferenceEngine::Ollama => write!(f, "ollama"),
            InferenceEngine::LlamaServer => write!(f, "llama_server"),
            InferenceEngine::OpenAi => write!(f, "openai"),
            InferenceEngine::Other(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    Chat,
    Embedding,
    ToolUse,
    JsonMode,
    Summarization,
}

impl std::fmt::Display for ModelCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelCapability::Chat => write!(f, "chat"),
            ModelCapability::Embedding => write!(f, "embedding"),
            ModelCapability::ToolUse => write!(f, "tool_use"),
            ModelCapability::JsonMode => write!(f, "json_mode"),
            ModelCapability::Summarization => write!(f, "summarization"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBackend {
    pub backend_id: String,
    pub engine: InferenceEngine,
    pub endpoint: String,
    pub model_id: String,
    pub node_id: String,
    pub sovereign: bool,
    pub capabilities: Vec<ModelCapability>,
    pub discovered_at: i64,
    pub last_seen: i64,
    pub health: ModelHealth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelHealth {
    Healthy,
    Degraded,
    Unreachable,
    Unknown,
}

impl std::fmt::Display for ModelHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelHealth::Healthy => write!(f, "healthy"),
            ModelHealth::Degraded => write!(f, "degraded"),
            ModelHealth::Unreachable => write!(f, "unreachable"),
            ModelHealth::Unknown => write!(f, "unknown"),
        }
    }
}
