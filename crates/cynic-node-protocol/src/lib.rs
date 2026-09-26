//! CYNIC Node Protocol (Domain Protocols & Schemas)
//!
//! # Focal Point
//! - **Responsibility**: Canonical domain contracts, pure schemas, and state transitions
//!   for node identity, enrollment, pre-OS lifecycle, activation, and control plane.
//! - **Business Logic**: Deterministic zero-I/O validation of node tokens, capability grants,
//!   hardware/machine fingerprints, and execution containment modes.
//! - **Production**: Machine-readable schemas and signed provenance receipts
//!   for node lifecycle events (Foundation §481, Tier 2 Protocol).

pub mod machine_enrollment;
pub mod node_activation;
pub mod node_control;
pub mod node_enrollment;
pub mod node_execution_mode;
pub mod node_identity;
pub mod pre_os;

pub use machine_enrollment::*;
pub use node_activation::*;
pub use node_control::*;
pub use node_enrollment::*;
pub use node_execution_mode::*;
pub use node_identity::*;
pub use pre_os::*;
