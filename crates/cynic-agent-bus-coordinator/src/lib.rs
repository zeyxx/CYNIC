//! CYNIC Agent Bus Coordinator — pyramid fleet orchestration.
//!
//! The coordinator is the executive branch of the computational government.
//! It decomposes plans into task DAGs and assigns them to domain leads.
//!
//! ## Military pyramid (span of control 3-7)
//!
//! ```
//! Human
//! └── Coordinator (1 per fleet)
//!     ├── Lead_Forensics ── Workers (3-7)
//!     ├── Lead_Kernel ── Workers (3-7)
//!     ├── Lead_Inference ── Workers (3-7)
//!     ├── Lead_Security ── Workers (3-7)
//!     ├── Lead_Protocol ── Workers (3-7)
//!     └── Lead_MCP ── Workers (3-7)
//! ```
//!
//! ## Rules
//!
//! - **Span of control**: 1 lead manages 3-7 workers max (Miller's law).
//! - **Subsidiarity**: decisions made at lowest possible level.
//! - **Failover**: if coordinator dies, oldest lead takes over.
//! - **Stagnation**: 3× identical output → escalate to coordinator.

pub mod coordinator;
pub mod taxonomy;

pub use coordinator::*;
pub use taxonomy::*;
