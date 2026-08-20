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

// Same treatment for `fleet_divergence`: without it, `cargo test --lib` cannot
// see the module at all, so its nine scan tests never ran — which is why a real
// fixture race in `write_registry` went unnoticed until it was validated out of
// tree. Declaring it here puts those tests inside the `make check` gate.
//
// `crate::cli_json` is the only intra-crate path it needs, and the lib already
// provides it above.
#[path = "fleet_divergence/mod.rs"]
pub mod fleet_divergence;
