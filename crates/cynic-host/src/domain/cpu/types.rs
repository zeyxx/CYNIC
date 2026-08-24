// Canonical CPU contract types are owned by `cynic_protocol`. This module
// only re-exports them so host-domain code has a stable path
// (`crate::domain::cpu::*`) without duplicating policy.
pub use cynic_protocol::cpu::*;
