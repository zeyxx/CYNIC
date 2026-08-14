//! NullPublisher — no-op ModelPublisherPort for tests and graceful degradation.

use async_trait::async_trait;

use crate::ports::model_publisher_port::{ModelPublisherPort, NormalisedModel, PublishResult};

#[derive(Debug)]
pub struct NullPublisher;

#[async_trait]
impl ModelPublisherPort for NullPublisher {
    fn name(&self) -> &str {
        "null"
    }

    async fn publish(&self, model: &NormalisedModel) -> PublishResult {
        PublishResult {
            publisher: self.name().to_string(),
            model_id: model.model_id.clone(),
            created: false,
            error: None,
        }
    }
}
