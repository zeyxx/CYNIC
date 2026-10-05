//! Re-exports canonical mesh registry types from cynic-network.
//! Authority: FCIS pattern and DDD extraction.
//!
//! This module is the single `cynic-node` boundary onto the canonical mesh
//! registry model. Every re-export here is a consumed one: a `pub(crate) use`
//! no in-crate code reaches is a second, divergent spelling of a domain type.

pub(crate) use cynic_network::{MeshPolicy, MeshRegistry, RegistryService};
