//! `cynic-cli` library surface — thin re-exports for integration tests.
//!
//! The bin (`main.rs`) is the CLI facade; this lib exposes only the scan
//! read-back surface for `cynic agent soc` projection + integration tests.
//! Business logic lives in `cynic-kernel` / `cynic-protocol` / `cynic-host`.

// `cli_json` + `agent_surface` are owned by `main.rs` (bin crate). Reuse them
// from the lib via `#[path]` so integration tests reach the scan read-back
// surface without duplicating it or inheriting the full CLI facade.
#[path = "cli_json.rs"]
mod cli_json;

#[path = "agent_surface/mod.rs"]
pub mod agent_surface;
