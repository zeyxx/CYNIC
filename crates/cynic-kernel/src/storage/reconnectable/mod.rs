//! Reconnectable storage — swappable StoragePort + CoordPort wrappers.
//!
//! The kernel boots with NullStorage when SurrealDB is unavailable.
//! These wrappers delegate all trait methods through a RwLock<Arc<dyn T>>,
//! allowing a background task to swap NullStorage → SurrealHttpStorage
//! without restarting the kernel.

pub mod coord;
pub mod node_ports;
pub mod reconnector;
pub mod storage;
pub mod storage_activity;
pub mod storage_crystals;

#[cfg(test)]
mod tests;

pub use coord::ReconnectableCoord;
pub use node_ports::{ReconnectableNodeControl, ReconnectableNodeEnrollment};
pub use reconnector::StorageReconnector;
pub use storage::ReconnectableStorage;
