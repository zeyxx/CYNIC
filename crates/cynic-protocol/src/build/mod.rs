pub mod action;
pub mod artefact;
pub mod authority;
pub mod bootstrap;
pub mod conformance;
pub mod exec_dto;
pub mod release;
pub mod route;

/// Build discovery / latency sub-domain (Cycle 3.2b partition).
pub mod build_discovery;
pub mod build_discovery_recipe;
pub mod build_latency;

/// Canonical node capability role for build runners (per `contracts/glossary.json`).
pub const BUILD_RUNNER_ROLE: &str = "build_runner";

pub use action::*;
pub use artefact::*;
pub use authority::*;
pub use bootstrap::*;
pub use conformance::*;
pub use exec_dto::*;
pub use release::*;
pub use route::*;
