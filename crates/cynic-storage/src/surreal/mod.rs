pub mod adapters;
pub mod domain;
pub mod ports;

pub use adapters::{SurrealConfig, SurrealHttpClient};
pub use domain::{
    AgentSessionRecord, AgentSessionStatus, EpistemicFactRecord, EpistemicStatus, SurrealQuery,
    SurrealQueryResult,
};
pub use ports::{SurrealHealthReport, SurrealStoragePort};
