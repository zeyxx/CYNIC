pub mod bootstrap;
pub mod discovery_candidate;
pub mod fleet_reconcile;
pub mod mesh;
pub mod mesh_coordination;
pub mod network_execution;
pub(crate) mod network_mesh_apply;
pub mod network_projection;
pub mod node_contribution;
pub mod node_observation;
pub mod observation_plan;
pub mod observation_runtime;
pub mod runtime_federation;
pub mod source_repository_proof;
pub mod storage_lifecycle;
pub mod supervision;
pub mod system_observation;
pub mod version;

// Pure build-decision logic (builder restart, build-runner selection /
// degradation) is implemented in cynic-build and re-exported here so that
// cynic-node stays decoupled from the build engine's selection surface.
pub use cynic_build::{builder_restart, domain::fleet_truth};
