//! Functional Core (Zero-I/O) domain logic, aggregates, and query builders.

pub mod memory;
pub mod query;
pub mod response;
pub mod session;
pub mod topology;

pub use memory::{EpistemicFactRecord, EpistemicStatus};
pub use query::SurrealQuery;
pub use response::{SurrealQueryResult, parse_and_validate_results};
pub use session::{AgentSessionRecord, AgentSessionStatus};
pub use topology::TopologicalContext;
