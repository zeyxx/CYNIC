//! cynic-types — Layer 0 atomic types shared across the CYNIC workspace.
//!
//! These are the foundational primitives depended on by ALL crates.
//! Zero I/O. Zero policy. Zero cross-crate dependencies upward.
//!
//! ## DAG position
//!
//! `cynic-types` is the root — it depends on NO cynic crate.
//! Every other crate may depend on `cynic-types`.
//!
//! ## Contents
//!
//! - [`AgentId`] — 128-bit agent identity (BLAKE3 of ed25519 pubkey)
//! - [`Hash32`] — 256-bit content hash (SHA-256 / BLAKE3)
//! - [`Budget`] — action count + wall-clock seconds
//! - [`Scope`] / [`ScopeKind`] — resource scope for claims and proofs
//! - [`TaskId`] — opaque task identifier
//! - [`Timestamp`] — Unix-seconds wall clock

pub mod agent;
pub mod budget;
pub mod hash;
pub mod scope;
pub mod task;
pub mod timestamp;

pub use agent::AgentId;
pub use budget::Budget;
pub use hash::Hash32;
pub use scope::{Scope, ScopeKind};
pub use task::TaskId;
pub use timestamp::Timestamp;

pub use serde::{Deserialize, Serialize};
