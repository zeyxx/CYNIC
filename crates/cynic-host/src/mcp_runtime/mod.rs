//! Read-only observation of the installed MCP runtime artifact.
//!
//! The MCP binary remains the authority for its compiled contract. This
//! module only executes its documented observation argv through an injected
//! port and projects the result for runtime/conformance consumers.

mod observe;
mod types;
mod webhook_receiver;

pub use observe::observe_mcp_runtime;
pub use types::{McpRuntimeObservation, MCP_RUNTIME_OBSERVATION_SCHEMA};
pub use webhook_receiver::{
    SettlementNotification, SETTLEMENT_NOTIFICATION_SCHEMA, WebhookSettlementMonitor,
};
