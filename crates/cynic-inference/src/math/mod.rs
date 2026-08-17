//! Measured mathematical primitives for inference configuration.
//!
//! - Roofline bounds from measured bandwidth and explicit per-token traffic
//! - Pareto-frontier extraction without scalar preference weights
//! - UCB1 repeat-benchmark scheduling over observed, normalized rewards
//! - Exact KV-cache storage from model tensor geometry

pub mod pareto;
pub mod roofline;
pub mod tensor_geometry;
pub mod ucb_sampler;

pub use pareto::{
    extract_pareto_frontier, select_utopian_optimal, ObjectiveBounds, ObjectiveVector,
};
pub use roofline::{
    assess_roofline, calculate_theoretical_max_tps, RooflineAssessment, RooflineBottleneck,
    RooflineInputs,
};
pub use tensor_geometry::ExactTensorGeometry;
pub use ucb_sampler::{select_ucb1_arm, ucb1_score, RewardError, Ucb1Arm};
