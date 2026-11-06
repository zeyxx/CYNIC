//! Host registry profiles, resolution, and runtime registry path discovery.
//!
//! Extracted from cynic-host to reduce the god-crate surface.

pub mod package_provenance;
pub mod harness_progression;
pub mod persist;
pub mod process_observation;
pub mod project_root;
pub mod runtime_authority;
pub mod runtime_layout;
pub mod runtime_network_preflight;
pub mod runtime_registry;
pub mod runtime_execution_context;
pub mod runtime_harness;
pub mod runtime_seed;
pub mod scan;
pub mod scan_exclusions;
pub mod types;
pub mod validate;
pub mod registry_divergence_probe;
pub mod registry_observation;
pub mod kernel_proof_persistence;
pub mod gc_surfaces;
pub use package_provenance::*;
pub use harness_progression::*;
pub use persist::*;
pub use process_observation::*;
pub use runtime_execution_context::*;
pub use project_root::current_project_root;
pub use runtime_authority::*;
pub use runtime_layout::*;
pub use runtime_registry::*;
pub use runtime_network_preflight::*;
pub use runtime_seed::*;
pub use scan::*;
pub use scan_exclusions::*;
pub use types::*;
pub use validate::*;
pub use registry_divergence_probe::*;
pub use registry_observation::*;
pub use runtime_harness::*;
pub use gc_surfaces::*;
pub use kernel_proof_persistence::*;
