//! CYNIC agent domain crate (L2).
//!
//! Unified autonomous agent traits and the offloaded operator, cleanup,
//! and merge agents.

pub mod cleanup;
pub mod merge_agent;
pub mod operator;
pub mod traits;

pub use cleanup::CleanupAgent;
pub use operator::OperatorAgent;
pub use traits::AutonomousAgent;