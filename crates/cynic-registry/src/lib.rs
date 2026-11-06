//! Host registry profiles, resolution, and runtime registry path discovery.
//!
//! Extracted from cynic-host to reduce the god-crate surface.

pub mod package_provenance;
pub mod persist;
pub mod process_observation;
pub mod project_root;
pub mod runtime_authority;
pub mod runtime_layout;
pub mod runtime_registry;
pub mod scan;
pub mod types;
pub mod validate;
pub mod registry_divergence_probe;
pub mod registry_observation;

pub use package_provenance::*;
pub use persist::*;
pub use process_observation::*;
pub use project_root::current_project_root;
pub use runtime_authority::*;
pub use runtime_layout::*;
pub use runtime_registry::resolve_runtime_registry_path;
pub use scan::*;
pub use types::*;
pub use validate::*;
pub use registry_divergence_probe::*;
pub use registry_observation::*;
