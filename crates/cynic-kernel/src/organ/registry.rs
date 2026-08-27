// registry.rs — Catalog of known Dog backends: capabilities and live state.
//
// Types canonical owner: cynic-inference::domain::backend_health (Slice A / Slice 1).
// This module re-exports them for full backward compatibility across the kernel.

pub use cynic_inference::backend_health::{
    Backend, BackendHealth, BackendId, DeclaredCapabilities, MeasuredCapabilities,
};
