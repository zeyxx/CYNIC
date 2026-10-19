//! Measured mathematical primitives for inference configuration.
//!
//! - Roofline bounds from measured bandwidth and explicit per-token traffic
//! - Pareto-frontier extraction without scalar preference weights
//! - UCB1 repeat-benchmark scheduling over observed, normalized rewards
//! - Exact KV-cache storage from model tensor geometry

pub mod roofline;
pub mod tensor_geometry;

pub use roofline::{
    RooflineAssessment, RooflineBottleneck, RooflineInputs, assess_roofline,
    calculate_theoretical_max_tps,
};
pub use tensor_geometry::ExactTensorGeometry;
