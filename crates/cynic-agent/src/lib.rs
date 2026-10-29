//! CYNIC agent domain crate (L2).
//!
//! Unified autonomous agent traits and the offloaded operator, cleanup,
//! and merge agents. Also absorbs the former `cynic-workload` bounded
//! context (workload, workspace, scheduler, skill, capability).

pub mod capability;
pub mod cleanup;
pub mod coordination;
pub mod merge_agent;
pub mod operator;
pub mod scheduler;
pub mod skill;
pub mod traits;
pub mod workload;
pub mod workspace;

pub use capability::*;
pub use cleanup::CleanupAgent;
pub use coordination::*;
pub use operator::OperatorAgent;
pub use scheduler::*;
pub use skill::*;
pub use traits::AutonomousAgent;
pub use workload::*;
pub use workspace::*;

#[cfg(test)]
mod tests;
