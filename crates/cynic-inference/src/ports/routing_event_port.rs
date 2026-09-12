//! Port for emitting routing and performance observation events.

use async_trait::async_trait;

#[async_trait]
pub trait RoutingEventPort: Send + Sync {
    async fn record_routing_observation(
        &self,
        domain: &str,
        dog_id: &str,
        latency_ms: u32,
        success: bool,
    ) -> Result<(), String>;
}
