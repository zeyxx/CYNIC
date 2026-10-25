//! Adapters for personal anchoring storage.
//!
//! Phase 1: in-memory adapter for tests.
//! Phase 2: SQLite/Room adapters.

pub mod memory;

pub use memory::InMemoryAnchorStore;