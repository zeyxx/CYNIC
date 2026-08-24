// Canonical network observation types are owned by `cynic_protocol::network`.
// This module only re-exports them so host-domain code has a stable path
// (`crate::domain::network::*`) without duplicating policy.
pub use cynic_protocol::network::*;
