//! ModelSyncPipeline orchestrates the discovery and publishing of local models.
//!
//! It probes local ports to find active services (Ollama, llama-server) and pushes
//! them to all configured targets (such as inference.toml, Open WebUI, etc.) in parallel.

use crate::ods_discovery::discover_ods_services;
use crate::ports::model_publisher_port::{ModelPublisherPort, NormalisedModel, PublishResult};
use std::fmt;
use std::sync::Arc;

/// Pipeline that runs discovery and publishes results to N target interfaces.
pub struct ModelSyncPipeline {
    publishers: Vec<Arc<dyn ModelPublisherPort>>,
    node_id: String,
}

impl ModelSyncPipeline {
    /// Create a new sync pipeline for the given node ID.
    pub fn new(node_id: String) -> Self {
        Self {
            publishers: Vec::new(),
            node_id,
        }
    }

    /// Add a target publisher to the pipeline.
    pub fn with_publisher(mut self, publisher: Arc<dyn ModelPublisherPort>) -> Self {
        self.publishers.push(publisher);
        self
    }

    /// Perform a discovery scan and publish the discovered models to all targets.
    pub async fn run_sync(&self) -> Vec<PublishResult> {
        let services = discover_ods_services().await;
        let mut results = Vec::new();

        for service in &services {
            for model_id in &service.models {
                let normalised = NormalisedModel::from_discovered(service, model_id, &self.node_id);
                for publisher in &self.publishers {
                    let res = publisher.publish(&normalised).await;
                    results.push(res);
                }
            }
        }

        results
    }
}

impl fmt::Debug for ModelSyncPipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelSyncPipeline")
            .field("publisher_count", &self.publishers.len())
            .field("node_id", &self.node_id)
            .finish()
    }
}
