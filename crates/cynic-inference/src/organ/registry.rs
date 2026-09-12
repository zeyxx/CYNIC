//! registry.rs — Re-exports canonical Backend health and capabilities from cynic-inference.
//!
//! SoC rule (foundation §481): cynic-inference owns backend health and capability types.

pub use cynic_inference::domain::backend_health::*;
