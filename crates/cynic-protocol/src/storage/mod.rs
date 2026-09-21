//! Storage contracts, controller, lifecycle, credential sync, identity, and topology.
//!
//! Sub-domain grouping:
//! - `client_policy`: storage access and mutation policy.
//! - `controller`: state machine and controller record validation.
//! - `credential_sync`: storage credential synchronization and drift detection.
//! - `identity`: storage endpoint identity and enrollment contracts.
//! - `lifecycle`: storage classes, volumes, claims, bindings, and resize planning.
//! - `topology`: storage hardware and network topology mapping.

pub mod client_policy;
pub mod controller;
pub mod credential_sync;
pub mod identity;
pub mod lifecycle;
pub mod topology;

pub use client_policy::*;
pub use controller::*;
pub use credential_sync::*;
pub use identity::*;
pub use lifecycle::*;
pub use topology::*;
