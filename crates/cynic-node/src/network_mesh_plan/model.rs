//! Re-exports canonical mesh registry types from cynic-network.
//! Authority: FCIS pattern and DDD extraction.
//!
//! This module is the single `cynic-node` boundary onto the canonical mesh
//! registry model, so a divergent in-crate copy of these types cannot grow.
//!
//! Visibility is deliberately split. `MeshRegistry` is the production read
//! path (`network_mesh_registry_port` plus `plan_from_registry`). The
//! remaining types are consumed only by the mesh plan tests, so under
//! `[workspace.lints] unused_imports = "deny"` a `pub(crate) use` of them
//! would fail `cargo check --lib`, which does not compile test code. They
//! live behind `cfg(any(test, kani))` so the production build sees only what
//! production reads. Nothing is lint-suppressed.

pub(crate) use cynic_network::MeshRegistry;

#[cfg(any(test, kani))]
pub(crate) use cynic_network::{
    MeshPolicy, NetworkPlane, NetworkPolicy, RegistryNode, RegistryService,
};
