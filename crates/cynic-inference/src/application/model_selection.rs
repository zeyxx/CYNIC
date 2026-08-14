use crate::domain::backend_identity::BackendHealthSnapshot;
use crate::domain::routing::{RoutingDecision, RoutingIntent};

use super::routing_policy::select_backend;

pub fn select_model(
    backends: &[BackendHealthSnapshot],
    intent: &RoutingIntent,
) -> Option<RoutingDecision> {
    select_backend(backends, intent)
}
