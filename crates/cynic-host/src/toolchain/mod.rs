//! Toolchain lifecycle: dynamic detection before specialization.
//!
//! Implements the bottom-up portable port that observes the local Rust
//! toolchain (rustc version, ICE artifacts, cache state), reconciles it
//! against the declarative registry in `infra/`, and produces a pure plan
//! to inject the right `RUSTUP_TOOLCHAIN` env.
//!
//! See CYNIC coherence pattern: desired state (registry) -> observe (no mutation)
//! -> reconcile (pure fn) -> bounded action (argv env inject) -> read-back proof.

pub mod observe;
pub mod policy;
pub mod port;
pub mod proof;
pub mod specs;

pub use observe::{ToolchainObservation, ToolchainObservationPort};
pub use port::{ToolchainLifecycle, ToolchainLifecycleReport};
pub use policy::{ToolchainDecision, reconcile_toolchain};
pub use specs::{ToolchainEntry, ToolchainProofHistory, ToolchainRegistry};
pub use proof::ToolchainProof;
