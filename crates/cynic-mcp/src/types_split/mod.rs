//! Subsplit of `crate::types` — parameter structs generated/moved here during the
//! MCP proxy surface modularisation. Declared explicitly (was orphan) so the
//! structs below are addressable from `crate::types` and `crates/cynic-mcp/src/routers`.
//!
//! Re-exported selectively from `crate::types` to avoid duplicate-definition
//! conflicts with the canonical `types.rs`.

pub mod coord;
pub mod core_params;
pub mod fleet;
pub mod identity;
pub mod inference;
pub mod infra;
pub mod network;
pub mod rate_limit;
pub mod solana_sop;
pub mod storage;
pub mod task;

#[cfg(test)]
mod tests;
