use crate::domain::backend_identity::BackendHealthSnapshot;
<<<<<<< HEAD
use crate::infra::routing::{RoutingDecision, RoutingIntent};
=======
use crate::domain::routing::{RoutingDecision, RoutingIntent};
>>>>>>> github/main

use super::routing_policy::select_backend;

pub fn select_model(
    backends: &[BackendHealthSnapshot],
    intent: &RoutingIntent,
) -> Option<RoutingDecision> {
    select_backend(backends, intent)
}
