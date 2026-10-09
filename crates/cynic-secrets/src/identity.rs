//! Identity types re-exported from the merged cynic-identity crate.
//!
//! This module groups all identity-related types (NodeId, NodeIdentity,
//! SigningMaterial, IdentityError) under a single `identity` namespace,
//! preserving the public API that cynic-identity consumers expect.

pub use crate::error::IdentityError;
pub use crate::types::{NodeId, NodeIdentity, SigningMaterial};
