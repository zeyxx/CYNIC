//! Hexagonal Port for executing Agentic Tool Calls (ATC).

use async_trait::async_trait;
use cynic_protocol::agentic_tool_call::{AgenticToolCallInput, AgenticToolCallResult};

/// Port for processing Agentic Tool Calls with pre-flight sanitization,
/// budget checking, safety evaluation, and proof generation.
#[async_trait]
pub trait AgenticToolCallPort: Send + Sync {
    async fn process_call(&self, input: AgenticToolCallInput) -> AgenticToolCallResult;
}
