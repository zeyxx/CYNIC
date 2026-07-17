use crate::domain::backend_identity::BackendHealthSnapshot;
use crate::domain::routing::{RoutingConstraint, RoutingDecision, RoutingIntent};

pub fn select_backend(
    backends: &[BackendHealthSnapshot],
    intent: &RoutingIntent,
) -> Option<RoutingDecision> {
    let requires_sovereign = intent
        .constraints
        .iter()
        .any(|constraint| matches!(constraint, RoutingConstraint::SovereignOnly));

    backends
        .iter()
        .find(|snapshot| {
            snapshot.status.is_available() && (!requires_sovereign || snapshot.identity.sovereign)
        })
        .map(|snapshot| RoutingDecision {
            backend_name: snapshot.identity.backend_name.clone(),
            reason: "first_available_backend".to_string(),
        })
}
