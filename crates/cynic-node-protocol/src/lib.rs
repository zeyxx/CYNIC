//! CYNIC Node Protocol (Domain Protocols & Schemas)
//!
//! # Focal Point
//! - **Responsibility**: Canonical domain contracts, pure schemas, and state transitions
//!   for node identity, enrollment, pre-OS lifecycle, activation, control plane,
//!   emergence, autonomy, federation, and deployment epoch.
//! - **Business Logic**: Deterministic zero-I/O validation of node tokens, capability grants,
//!   hardware/machine fingerprints, runtime takeover, and federation reconciliation.
//! - **Production**: Machine-readable schemas and signed provenance receipts
//!   for node lifecycle events (Foundation §481, Tier 2 Protocol).

pub mod bootstrap_federation;
pub mod deployment_epoch;
pub mod emergence;
pub mod federation;
pub mod local_autonomy;
pub mod machine_enrollment;
pub mod node_activation;
pub mod node_control;
pub mod node_enrollment;
pub mod node_execution_mode;
pub mod node_identity;
pub mod peer_resolver;
pub mod pre_os;
pub mod runtime_takeover_activation;

pub use bootstrap_federation::*;
pub use deployment_epoch::*;
pub use emergence::*;
pub use federation::*;
pub use local_autonomy::*;
pub use machine_enrollment::*;
pub use node_activation::*;
pub use node_control::*;
pub use node_enrollment::*;
pub use node_execution_mode::*;
pub use node_identity::*;
pub use peer_resolver::*;
pub use pre_os::*;
pub use runtime_takeover_activation::*;
