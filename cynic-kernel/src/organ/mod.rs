// organ/ — InferenceOrgan: routes inference requests to Dogs, tracks health,
// exposes metrics, and enforces capacity/circuit-breaker policies.
// Phase 1 placeholder — types and logic added in subsequent tasks.

pub mod health;
pub mod metrics;
pub mod registry;
pub mod router;
pub mod transport;
