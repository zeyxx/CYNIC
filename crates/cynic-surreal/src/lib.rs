//! `cynic-surreal` — Pure FCIS/DDD SurrealDB multi-model projection and epistemic memory crate.
//!
//! # Architecture (Strict FCIS / DDD)
//! - `domain/`: Functional Core — Zero-I/O pure queries, aggregates, and response validators.
//! - `ports/`: Abstract trait interfaces for SurrealDB operations.
//! - `adapters/`: Imperative Shell — Side-effects, HTTP REST calls, and credential resolution.

pub mod adapters;
pub mod domain;
pub mod ports;

pub use adapters::{SurrealConfig, SurrealHttpClient};
pub use domain::{
    AgentSessionRecord, AgentSessionStatus, EpistemicFactRecord, EpistemicStatus, SurrealQuery,
    SurrealQueryResult,
};
pub use ports::{SurrealHealthReport, SurrealStoragePort};
