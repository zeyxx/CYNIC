//! cynic-release: pure decision engine for release gates and relay authorization.
//!
//! Hexagonal domain layer:
//! - types: vocabulary of releases, verdicts, and proofs
//! - contracts: deserialization of JSON contracts
//! - evaluator: pure business evaluation (zero I/O)
//! - ports: driven interfaces for execution layers

pub mod adapters;
pub mod artifact_publication;
pub mod build_proof;
pub mod contracts;
pub mod deploy;
pub mod deploy_locations;
pub mod deploy_orchestrator;
pub mod deploy_remote;
pub mod deploy_ssh;
pub mod error;
pub mod evaluator;
pub mod external_deploy;
pub mod kernel_release;
pub mod ports;
pub mod types;

pub use adapters::*;
pub use build_proof::*;
pub use contracts::*;
pub use deploy::*;
pub use deploy_locations::*;
pub use deploy_orchestrator::*;
pub use deploy_remote::*;
pub use deploy_ssh::*;
pub use error::*;
pub use evaluator::*;
pub use external_deploy::*;
pub use ports::*;
pub use types::*;
