//! Runtime operations for inference node control/probing.
//!
//! This module now delegates to `cynic-inference` so that the operational
//! primitives can be reused directly by other surfaces while preserving the
//! `cynic-node` application façade expected by downstream surfaces/tests.

pub use cynic_inference::application::node_runtime_ops::{
    DogNodeBinding, DogOperationOutcome, ModelProbeResult, attempt_node_recovery,
    attempt_node_start, dog_to_node, probe_model_runtime, probe_node_model, remediate_dog_runtime,
    start_dog_runtime,
};
