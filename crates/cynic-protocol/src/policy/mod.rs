//! Policy sub-domain (DDD partition).
//!
//! Groups the policy-related protocol modules under a single namespace while
//! preserving the top-level `policy`, `publication_policy`, and `service_policy`
//! module paths for backward compatibility.

pub mod foundation;
pub use foundation::*;

pub use crate::publication_policy;
pub use crate::service_policy;
