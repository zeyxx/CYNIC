use super::types::{HarnessSample, ScenarioSpec, TaskSpec};
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationReceipt {
    pub coordination_kind: String,
    pub coordination_id: String,
    pub correlation_id: String,
    pub observation: cynic_protocol::CoordinationTaskObservation,
}

#[async_trait]
pub trait CoordinationPort: Send + Sync {
    async fn observe_or_create(
        &self,
        correlation_id: &str,
        coordination_id: Option<&str>,
        task: &super::types::TaskSpec,
    ) -> Result<CoordinationReceipt, String>;
    async fn complete(&self, receipt: &CoordinationReceipt, summary: &str) -> Result<(), String>;
    async fn block(&self, receipt: &CoordinationReceipt, reason: &str) -> Result<(), String>;
    async fn heartbeat(&self, receipt: &CoordinationReceipt) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct InferenceReply {
    pub runtime_run_id: Option<String>,
    pub http_status: Option<u16>,
    pub prompt_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub prompt_tokens_per_second: Option<f32>,
    pub output_tokens_per_second: Option<f32>,
    pub wall_time_ms: Option<u64>,
    pub event_count: Option<u32>,
    pub event_kinds: Vec<String>,
    pub event_stream_error: Option<String>,
    pub text_response: Option<String>,
    pub error: Option<String>,
}

#[async_trait]
pub trait HarnessExecutor: Send + Sync {
    async fn execute(&self, scenario: &ScenarioSpec, task: &TaskSpec) -> InferenceReply;
}

pub trait HarnessObserver: Send + Sync {
    fn observe(&self) -> super::types::MemorySample;
}

impl
    From<(
        String,
        InferenceReply,
        super::types::MemorySample,
        Option<u32>,
    )> for HarnessSample
{
    fn from(
        (task_id, reply, memory, effective_context_per_slot): (
            String,
            InferenceReply,
            super::types::MemorySample,
            Option<u32>,
        ),
    ) -> Self {
        let proof_error = reply
            .event_stream_error
            .clone()
            .map(|error| format!("event_stream:{error}"));
        let error = reply.error.or(proof_error);
        let status = if error.is_some() { "failed" } else { "ok" };
        Self {
            task_id,
            runtime_run_id: reply.runtime_run_id,
            http_status: reply.http_status,
            prompt_tokens: reply.prompt_tokens,
            output_tokens: reply.output_tokens,
            prompt_tokens_per_second: reply.prompt_tokens_per_second,
            output_tokens_per_second: reply.output_tokens_per_second,
            wall_time_ms: reply.wall_time_ms,
            event_count: reply.event_count,
            event_kinds: reply.event_kinds,
            event_stream_error: reply.event_stream_error,
            memory,
            effective_context_per_slot,
            status: status.into(),
            error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::agent_harness::types::MemorySample;

    #[test]
    fn missing_sse_proof_is_not_reported_as_success() {
        let sample: HarnessSample = (
            "task".into(),
            InferenceReply {
                runtime_run_id: Some("run_1".into()),
                http_status: Some(200),
                prompt_tokens: Some(1),
                output_tokens: Some(1),
                prompt_tokens_per_second: None,
                output_tokens_per_second: None,
                wall_time_ms: Some(1),
                event_count: None,
                event_kinds: Vec::new(),
                event_stream_error: Some("sse_timeout".into()),
                text_response: None,
                error: None,
            },
            MemorySample::default(),
            None,
        )
            .into();
        assert_eq!(sample.status, "failed");
        assert!(sample.error.unwrap().contains("event_stream:sse_timeout"));
    }
}
