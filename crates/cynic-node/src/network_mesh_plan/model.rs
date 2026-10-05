//! Re-exports canonical mesh registry types from `cynic-network`.
//!
//! Authority: FCIS pattern and DDD extraction — one model, one crate. This
//! module used to redeclare the registry structs locally, so the same JSON
//! was parsed twice into two incompatible types (the duplicate `MeshRegistry`
//! in `cynic-node` vs the canonical one in `cynic-network`).
pub(crate) use cynic_network::{
    MeshPolicy, MeshRegistry, NetworkPlane, NetworkPolicy, NetworkReservation, RegistryNode,
    RegistryService,
};
