//! cynic-release: pure decision engine for release gates and relay authorization.
//!
//! Hexagonal domain layer:
//! - types: vocabulary of releases, verdicts, and proofs
//! - contracts: deserialization of JSON contracts
//! - evaluator: pure business evaluation (zero I/O)
//! - ports: driven interfaces for execution layers

pub mod contracts;
pub mod error;
pub mod evaluator;
pub mod ports;
pub mod types;

pub use contracts::*;
pub use error::*;
pub use evaluator::*;
pub use ports::*;
pub use types::*;
