//! Branch graph domain (git refs observation).
//!
//! Pure parsers live in `probe`; the observe port is in `port`; canonical
//! types are re-exported from `cynic_protocol::branch_graph`.

pub mod port;
pub mod probe;

pub use cynic_protocol::branch_graph::{
    BRANCH_GRAPH_SCHEMA, BRANCH_STALE_AFTER_SECS, BranchGraph, BranchNode, BranchObservationPort,
    is_root, is_stale,
};
pub use port::{BranchGraphObservationError, BranchGraphObservationPort};
pub use probe::{
    enrich_with_parents, parse_git_for_each_ref, parse_git_log_graph, stale_via_policy,
};
