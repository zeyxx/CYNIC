use async_trait::async_trait;

use crate::domain::chat::{ChatError, ChatResponse, InferenceProfile};
use crate::ports::inference_port::BackendPort;

#[async_trait]
pub trait ChatPort: BackendPort {
    async fn chat(
        &self,
        system: &str,
        user: &str,
        profile: InferenceProfile,
        request_id: Option<&str>,
    ) -> Result<ChatResponse, ChatError>;
}

/// Mock implementation for tests.
#[derive(Debug)]
pub struct MockChatBackend {
    pub response: String,
    pub reasoning: Option<String>,
    pub name: String,
    pub force_error: Option<ChatError>,
}

impl MockChatBackend {
    pub fn new(name: &str, response: &str) -> Self {
        Self {
            response: response.to_string(),
            reasoning: None,
            name: name.to_string(),
            force_error: None,
        }
    }
}

#[async_trait]
impl BackendPort for MockChatBackend {
    fn name(&self) -> &str {
        &self.name
    }

    async fn health(&self) -> crate::domain::inference::BackendStatus {
        if self.force_error.is_some() {
            crate::domain::inference::BackendStatus::Critical
        } else {
            crate::domain::inference::BackendStatus::Healthy
        }
    }
}

#[async_trait]
impl ChatPort for MockChatBackend {
    async fn chat(
        &self,
        _system: &str,
        _user: &str,
        _profile: InferenceProfile,
        _request_id: Option<&str>,
    ) -> Result<ChatResponse, ChatError> {
        if let Some(ref err) = self.force_error {
            return Err(err.clone());
        }
        Ok(ChatResponse {
            text: self.response.clone(),
            reasoning: self.reasoning.clone(),
            prompt_tokens: 0,
            completion_tokens: 0,
            thinking_tokens: 0,
        })
    }
}
