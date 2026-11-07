//! Fleet & federation lifecycle sub-domain (DDD partition).
//!
//! Groups the fleet and federation protocol modules under a single namespace while
//! preserving the top-level `fleet_*` and `federation_*` module paths for backward
//! compatibility.

pub use crate::federation_authz;
pub use crate::federation_envelopes;
pub use crate::federation_membership;

/// Fleet discovery / heartbeat / target sub-domain (Cycle 3.2b partition).
pub mod fleet_discovery;
pub mod fleet_heartbeat;
pub mod fleet_target;

pub mod consensus;
pub use consensus::*;