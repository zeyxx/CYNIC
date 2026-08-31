//! organ/ — InferenceOrgan: owns backend registry, health tracking, and routing data.
//!
//! Canonical types live in `cynic_inference::domain::backend_health` (§481).
//! Organ internal structures are modularized into bounded components (<300 LOC).

pub mod backend_handle;
pub mod health;
pub mod inference_organ;
pub mod registry;

#[cfg(test)]
mod tests;

pub use backend_handle::{BackendEntry, BackendHandle};
pub use health::ParseFailureGate;
pub use inference_organ::InferenceOrgan;
pub use registry::{Backend, BackendHealth, BackendId, DeclaredCapabilities, MeasuredCapabilities};
