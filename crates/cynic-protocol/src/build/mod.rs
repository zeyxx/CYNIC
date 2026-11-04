pub mod action;
pub mod artefact;
pub mod authority;
pub mod bootstrap;
pub mod conformance;
pub mod exec_dto;
pub mod release;
pub mod route;

/// Backward-compatible re-exports of the flat build sub-domain modules, so
/// callers can reach them via `cynic_protocol::build::*`.
pub use crate::build_discovery;
pub use crate::build_discovery_recipe;
pub use crate::build_latency;

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
