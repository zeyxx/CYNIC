//! Re-exports for builder HTTP server and restart logic.
//!
//! Historical location: `cynic_node::usecases::builder_http`.
//! Under FCIS:
//! - Pure domain decision logic lives in `cynic_node::domain::builder_restart`.
//! - Axum HTTP server and endpoints live in `cynic_node::adapters::http_builder_server`.

pub use crate::adapters::http_builder_server::*;
pub use crate::domain::builder_restart::*;
