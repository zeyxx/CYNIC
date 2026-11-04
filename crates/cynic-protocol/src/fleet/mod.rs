//! Fleet & federation lifecycle sub-domain (DDD partition).
//!
//! Groups the fleet and federation protocol modules under a single namespace while
//! preserving the top-level `fleet_*` and `federation_*` module paths for backward
//! compatibility.

pub use crate::federation_authz;
pub use crate::federation_envelopes;
pub use crate::federation_membership;
pub use crate::fleet_discovery;
pub use crate::fleet_heartbeat;
pub use crate::fleet_target;