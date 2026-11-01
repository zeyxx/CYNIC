pub mod action;
pub mod artefact;
pub mod authority;
pub mod bootstrap;
pub mod conformance;
pub mod exec_dto;
pub mod release;
pub mod route;

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
