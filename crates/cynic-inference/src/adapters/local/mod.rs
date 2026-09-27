//! Native CYNIC local inference adapters (vault + process supervisor).
//!
//! Part of **our** system: declared profiles + `llama-server@` (or equivalent).
//! Not a substitute for [Osmantic ODS](https://github.com/Osmantic/ODS), which
//! remains the preferred full-environment bootstrap framework when installed.
//! ODS integration (observe running stack) lives in `adapters::ods`.

pub mod docker_supervisor;
pub mod engine_capability_store;
pub mod engine_probe;
pub mod host_probe;
pub mod model_store;
pub mod process_supervisor;
pub mod session_store;
pub mod systemd_user;
pub mod vault_inventory;

pub use docker_supervisor::DockerLocalRuntime;
pub use engine_capability_store::{
    EngineCapabilities, EngineCapabilityStore, Observation, PreflightDecision,
};
pub use host_probe::{FsHardwareProbe, probe_hardware};
pub use model_store::FsModelStore;
pub use process_supervisor::{InferenceSupervisorKind, ProcessLocalRuntime};
pub use session_store::FsChatSessionStore;
pub use systemd_user::SystemdUserLocalRuntime;
pub use vault_inventory::FsLocalModelInventory;
pub mod filesystem_capacity;
