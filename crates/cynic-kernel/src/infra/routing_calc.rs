//! Dynamic routing calculator — re-exported from `cynic-inference` per Foundation §481 (SoC).

pub use cynic_inference::routing_calc::*;
use std::sync::Arc;

use crate::domain::storage::{Event, StoragePort};
use crate::infra::config::BackendConfig;

pub trait RoutingCalculatorStorageExt {
    fn with_storage_and_backends(
        storage: Arc<dyn StoragePort>,
        backends: &[BackendConfig],
    ) -> RoutingCalculator;
}

struct KernelStorageRoutingEventAdapter(Arc<dyn StoragePort>);

#[async_trait::async_trait]
impl cynic_inference::ports::routing_event_port::RoutingEventPort
    for KernelStorageRoutingEventAdapter
{
    async fn record_routing_observation(
        &self,
        domain: &str,
        dog_id: &str,
        latency_ms: u32,
        success: bool,
    ) -> Result<(), String> {
        let event = Event {
            event_type: "dog_performance".to_string(),
            domain: domain.to_string(),
            data: serde_json::json!({
                "dog_id": dog_id,
                "domain": domain,
                "latency_ms": latency_ms,
                "success": success,
            }),
            timestamp: chrono::Utc::now().timestamp(),
        };
        self.0.store_event(&event).await.map_err(|e| e.to_string())
    }
}

impl RoutingCalculatorStorageExt for RoutingCalculator {
    fn with_storage_and_backends(
        storage: Arc<dyn StoragePort>,
        backends: &[BackendConfig],
    ) -> RoutingCalculator {
        let adapter = Arc::new(KernelStorageRoutingEventAdapter(storage));
        let mut calc = RoutingCalculator::with_event_port(adapter);

        let mut dog_nodes = std::collections::HashMap::new();
        for b in backends {
            if let Some(node) = &b.node {
                dog_nodes.insert(b.id.clone(), node.clone());
            }
        }
        calc.set_dog_nodes(dog_nodes);
        calc
    }
}
