//! Branch graph observation port (pure observation).

use cynic_protocol::branch_graph::BranchGraph;

pub type BranchGraphObservationError = String;

pub trait BranchGraphObservationPort {
    fn observe_branch_graph(&self) -> Result<BranchGraph, BranchGraphObservationError>;
}
