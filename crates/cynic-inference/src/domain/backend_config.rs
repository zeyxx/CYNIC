//! Kernel-owned inference backend configuration types.
//! Extracted from cynic-inference to eliminate upward dependency.

use serde::{Deserialize, Serialize};

/// Which wire protocol the adapter should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendType {
    /// OpenAI-compatible HTTP API (llama.cpp, vLLM, Ollama, …)
    #[default]
    OpenAi,
    /// CLI subprocess (e.g. `gemini` binary)
    Cli,
}

/// How the adapter authenticates to the backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthStyle {
    /// `Authorization: Bearer *** header.
    Bearer,
    /// Key appended as a query parameter: `?<param>=<key>`.
    QueryParam { param: String },
    /// No authentication (typical for local sovereign backends).
    None,
}

/// Full configuration for one inference backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceBackendConfig {
    /// Stable identifier used as routing key and in logs.
    pub name: String,
    /// Wire protocol for this backend.
    #[serde(default)]
    pub backend_type: BackendType,
    /// Base URL of the inference server, e.g. `http://localhost:8080/v1`.
    pub base_url: String,
    /// API key. Leave unset for sovereign (no-auth) backends.
    #[serde(default)]
    pub api_key: Option<String>,
    /// Model identifier forwarded to the server.
    pub model: String,
    /// Authentication style. Defaults to `none` for sovereign backends.
    #[serde(default = "AuthStyle::default_none")]
    pub auth_style: AuthStyle,
    /// Advertised context window size in tokens (informational).
    #[serde(default)]
    pub context_size: u32,
    /// Per-request HTTP timeout in seconds.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// Maximum tokens to request from the backend.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Default sampling temperature.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Optional remote node that hosts this backend (for start/remediate/probe).
    #[serde(default)]
    pub fleet_node: Option<String>,
    /// Disable extended thinking/reasoning tokens (local models only).
    #[serde(default)]
    pub disable_thinking: bool,
    /// Request JSON-mode output from the server.
    #[serde(default)]
    pub json_mode: bool,
    /// Whether this backend supports tool/function calling.
    #[serde(default)]
    pub supports_tool_use: bool,
    /// Override health-check URL (defaults to `base_url/models`).
    #[serde(default)]
    pub health_url: Option<String>,
    /// Extra CLI arguments forwarded to `CliBackend` subprocess.
    #[serde(default)]
    pub cli_extra_args: Vec<String>,
    /// True ↔ this backend runs locally under CYNIC sovereign control.
    #[serde(default)]
    pub sovereign: bool,
}

impl AuthStyle {
    fn default_none() -> Self {
        Self::None
    }
}

fn default_timeout_secs() -> u64 {
    120
}
fn default_max_tokens() -> u32 {
    4096
}
fn default_temperature() -> f32 {
    0.7
}
