//! inference_routing_port.rs — Typed ports for model routing, fleet selection, and proxy dispatch.
//!
//! Separates kernel transport from domain routing and model selection logic (OWN-010 / Slice 4).

use crate::application::fleet_routing::{
    FleetNodeStat, InferenceCandidateView, InferenceRouteDecisionView, project_candidate_views,
    project_route_response,
};
use crate::application::node_runtime_ops::{DogNodeBinding, dog_to_node};
use crate::application::proxy_routing::{ProxyTargetSelectionError, SelectedProxyTarget};

/// Port for selecting optimal execution targets across available nodes and backends.
pub trait InferenceRoutingPort: Send + Sync {
    /// Selects the best candidate node for an inference route request based on observed stats.
    fn select_route(&self, stats: Vec<FleetNodeStat>) -> Option<InferenceRouteDecisionView>;

    /// Lists candidate views ranked by quality and latency.
    fn candidate_views(&self, stats: Vec<FleetNodeStat>, limit: usize) -> Vec<InferenceCandidateView>;

    /// Resolves a dog name to target node name using bindings.
    fn resolve_dog_node(&self, dog: &str, bindings: &[DogNodeBinding]) -> String;
}

/// Port for selecting proxy forward targets with slot and tool awareness.
pub trait InferenceProxyPort: Send + Sync {
    /// Resolves target backend/model endpoint under slot tracker constraints.
    fn select_target(
        &self,
        requested_backend: Option<&str>,
        requested_model: Option<&str>,
        requires_tools: bool,
    ) -> Result<SelectedProxyTarget, ProxyTargetSelectionError>;
}

/// Default pure adapter implementing `InferenceRoutingPort`.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultInferenceRouter;

impl InferenceRoutingPort for DefaultInferenceRouter {
    fn select_route(&self, stats: Vec<FleetNodeStat>) -> Option<InferenceRouteDecisionView> {
        project_route_response(stats)
    }

    fn candidate_views(&self, stats: Vec<FleetNodeStat>, limit: usize) -> Vec<InferenceCandidateView> {
        project_candidate_views(stats, limit)
    }

    fn resolve_dog_node(&self, dog: &str, bindings: &[DogNodeBinding]) -> String {
        dog_to_node(dog, bindings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_inference_router_select_route() {
        let router = DefaultInferenceRouter;
        let stats = vec![FleetNodeStat {
            node: "node-a".to_string(),
            avg_latency_ms: 100,
            success_rate: 0.95,
            total_requests: 100,
            error_count: 5,
        }];
        let decision = router.select_route(stats);
        assert!(decision.is_some());
        let route = decision.unwrap();
        assert_eq!(route.selected_node, "node-a");
    }

    #[test]
    fn test_default_inference_router_candidate_views() {
        let router = DefaultInferenceRouter;
        let stats = vec![FleetNodeStat {
            node: "node-a".to_string(),
            avg_latency_ms: 100,
            success_rate: 0.95,
            total_requests: 100,
            error_count: 5,
        }];
        let candidates = router.candidate_views(stats, 5);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].node, "node-a");
    }
}
