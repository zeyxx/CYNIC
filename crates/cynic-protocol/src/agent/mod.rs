//! Agent lifecycle sub-domain (DDD partition).
//!
//! Groups the agent-related protocol modules under a single namespace while
//! preserving the top-level `agent_*` module paths for backward compatibility.

pub use crate::agent_citizenship;
pub use crate::agent_connection;
pub use crate::agent_coordination;
pub use crate::agent_coordination_continuity;
pub use crate::agent_host_sandbox;
pub use crate::agent_isolation;
pub use crate::agent_mutation_gates;
pub use crate::agent_recursion;
pub use crate::agent_review;
pub use crate::agent_session;
