//! Pure fleet inference placement and rollout reconciliation.
//!
//! This module deliberately stops before transport, process spawning, DNS file
//! mutation, or model download. Those are node-owned adapters. The durable
//! contract decides *which* attested nodes may receive a bounded action and
//! when service DNS must wait for post-action health read-back.

mod policy;
mod types;

pub use policy::reconcile_fleet_inference;
pub use types::{
    FLEET_INFERENCE_DEPLOYMENT_SCHEMA, FLEET_INFERENCE_OBSERVATION_SCHEMA,
    FLEET_INFERENCE_PLAN_SCHEMA, FleetInferenceDeployment, FleetInferenceDnsAction,
    FleetInferenceDnsRecord, FleetInferenceNodeObservation, FleetInferencePlan,
    FleetInferenceRuntime, FleetInferenceTargetAction, FleetInferenceTargetPlan,
};
