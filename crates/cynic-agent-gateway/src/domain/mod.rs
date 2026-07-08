use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentExecutionEnvelope {
    pub task_id: String,
    pub domain: String,
    pub provider: String,
    pub transport: Transport,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transport {
    Gateway { endpoint: String },
    Cli { binary: String, default_args: Vec<String> },
    Mcp { server_name: String },
}
