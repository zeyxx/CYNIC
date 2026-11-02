//! Pure parsers for branch graph observation (no I/O) — re-exported from canonical owner cynic-protocol.
pub use cynic_protocol::branch_graph::{
    enrich_with_parents, parse_git_for_each_ref, parse_git_log_graph, stale_via_policy,
};
