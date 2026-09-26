//! cynic-core-domains: extracted kernel domains (R7 Phase 1).
//!
//! Pure domain logic — ccm intake, metrics, dog, storage policy.
//! Does NOT depend downward on cynic-host/node/cli (R3 strict DAG).

pub mod ccm;
pub mod domain;

pub use ccm::*;
pub use domain::*;
