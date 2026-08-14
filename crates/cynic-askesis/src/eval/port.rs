use async_trait::async_trait;

use crate::eval::types::{
    CompletionRequest, ModelCompletion, ToolCall, ToolDefinition, ToolExecution,
};

#[async_trait]
pub trait CompletionPort: Send + Sync {
    fn endpoint(&self) -> &str;
    fn model(&self) -> &str;
    async fn complete(&self, request: &CompletionRequest) -> crate::Result<ModelCompletion>;
}

#[async_trait]
pub trait ToolExecutionPort: Send + Sync {
    async fn execute(&self, call: &ToolCall, tools: &[ToolDefinition]) -> ToolExecution;
}

#[derive(Debug, Default)]
pub struct FixtureToolExecutor;

#[async_trait]
impl ToolExecutionPort for FixtureToolExecutor {
    async fn execute(&self, call: &ToolCall, tools: &[ToolDefinition]) -> ToolExecution {
        let arguments = serde_json::from_str(&call.arguments_json)
            .unwrap_or_else(|_| serde_json::json!({"invalid_json": call.arguments_json}));
        match tools.iter().find(|tool| tool.name == call.name) {
            Some(tool) => ToolExecution {
                call_id: call.id.clone(),
                tool: call.name.clone(),
                arguments,
                result: tool.fixture_result.clone(),
                status: "ok".into(),
            },
            None => ToolExecution {
                call_id: call.id.clone(),
                tool: call.name.clone(),
                arguments,
                result: serde_json::json!({"error": "unknown_fixture_tool"}),
                status: "rejected".into(),
            },
        }
    }
}
