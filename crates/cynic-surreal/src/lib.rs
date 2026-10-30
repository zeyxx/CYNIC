//! cynic-surreal: L1 SurrealDB domain — pure domain logic, ports, and HTTP adapter.

pub mod adapters;
pub mod domain;
pub mod ports;

pub use adapters::{SurrealConfig, SurrealHttpClient};
pub use domain::{
    AgentSessionRecord, AgentSessionStatus, EpistemicFactRecord, EpistemicStatus, SurrealQuery,
    SurrealQueryResult, TopologicalContext, parse_and_validate_results,
};
pub use ports::{SurrealHealthReport, SurrealStoragePort};
